use std::{backtrace::Backtrace, io::Write};

use turbopath::{AbsoluteSystemPath, AbsoluteSystemPathBuf, AnchoredSystemPathBuf};
use turborepo_api_client::{APIClient, Response};

use crate::{
    cache_archive::{CacheReader, CacheWriter},
    signature_authentication::ArtifactSignatureAuthenticator,
    CacheError, CacheOpts, CacheResponse, CacheSource,
};

pub struct HttpCache {
    client: APIClient,
    signer_verifier: Option<ArtifactSignatureAuthenticator>,
    repo_root: AbsoluteSystemPathBuf,
    token: String,
}

impl HttpCache {
    pub fn new(
        client: APIClient,
        opts: CacheOpts,
        repo_root: AbsoluteSystemPathBuf,
        team_id: &str,
        token: &str,
    ) -> HttpCache {
        let signer_verifier = if opts
            .remote_cache_opts
            .map_or(false, |remote_cache_opts| remote_cache_opts.signature)
        {
            Some(ArtifactSignatureAuthenticator {
                team_id: team_id.as_bytes().to_vec(),
                secret_key_override: None,
            })
        } else {
            None
        };

        HttpCache {
            client,
            signer_verifier,
            repo_root,
            token: token.to_string(),
        }
    }

    pub async fn put(
        &self,
        anchor: &AbsoluteSystemPath,
        hash: &str,
        files: &[AnchoredSystemPathBuf],
        duration: u32,
        token: &str,
    ) -> Result<(), CacheError> {
        let mut artifact_body = Vec::new();
        self.write(&mut artifact_body, anchor, files).await?;

        let tag = self
            .signer_verifier
            .as_ref()
            .map(|signer| signer.generate_tag(hash.as_bytes(), &artifact_body))
            .transpose()?;

        self.client
            .put_artifact(hash, &artifact_body, duration, tag.as_deref(), token)
            .await?;

        Ok(())
    }

    async fn write(
        &self,
        writer: impl Write,
        anchor: &AbsoluteSystemPath,
        files: &[AnchoredSystemPathBuf],
    ) -> Result<(), CacheError> {
        let mut cache_archive = CacheWriter::from_writer(writer, true)?;
        for file in files {
            cache_archive.add_file(anchor, &file)?;
        }

        Ok(())
    }

    pub async fn exists(
        &self,
        hash: &str,
        token: &str,
        team_id: &str,
        team_slug: Option<&str>,
        use_preflight: bool,
    ) -> Result<CacheResponse, CacheError> {
        let response = self
            .client
            .artifact_exists(hash, token, team_id, team_slug, use_preflight)
            .await?;

        let duration = Self::get_duration_from_response(&response)?;

        Ok(CacheResponse {
            source: CacheSource::Remote,
            time_saved: duration,
        })
    }

    fn get_duration_from_response(response: &Response) -> Result<u32, CacheError> {
        if let Some(duration_value) = response.headers().get("x-artifact-duration") {
            let duration = duration_value
                .to_str()
                .map_err(|_| CacheError::InvalidDuration(Backtrace::capture()))?;

            duration
                .parse::<u32>()
                .map_err(|_| CacheError::InvalidDuration(Backtrace::capture()))
        } else {
            Ok(0)
        }
    }

    pub async fn retrieve(
        &self,
        hash: &str,
        token: &str,
        team_id: &str,
        team_slug: Option<&str>,
        use_preflight: bool,
    ) -> Result<(CacheResponse, Vec<AnchoredSystemPathBuf>), CacheError> {
        let response = self
            .client
            .fetch_artifact(hash, token, team_id, team_slug, use_preflight)
            .await?;

        let duration = Self::get_duration_from_response(&response)?;

        let body = if let Some(signer_verifier) = &self.signer_verifier {
            let expected_tag = response
                .headers()
                .get("x-artifact-tag")
                .ok_or(CacheError::ArtifactTagMissing(Backtrace::capture()))?;

            let expected_tag = expected_tag
                .to_str()
                .map_err(|_| CacheError::InvalidTag(Backtrace::capture()))?
                .to_string();

            let body = response.bytes().await.map_err(|e| {
                CacheError::ApiClientError(
                    Box::new(turborepo_api_client::Error::ReqwestError(e)),
                    Backtrace::capture(),
                )
            })?;
            let is_valid = signer_verifier.validate(hash.as_bytes(), &body, &expected_tag)?;

            if !is_valid {
                return Err(CacheError::InvalidTag(Backtrace::capture()));
            }

            body
        } else {
            response.bytes().await.map_err(|e| {
                CacheError::ApiClientError(
                    Box::new(turborepo_api_client::Error::ReqwestError(e)),
                    Backtrace::capture(),
                )
            })?
        };

        let files = Self::restore_tar(&self.repo_root, &body)?;

        Ok((
            CacheResponse {
                source: CacheSource::Remote,
                time_saved: duration,
            },
            files,
        ))
    }

    pub(crate) fn restore_tar(
        root: &AbsoluteSystemPath,
        body: &[u8],
    ) -> Result<Vec<AnchoredSystemPathBuf>, CacheError> {
        let mut cache_reader = CacheReader::from_reader(body, true)?;
        cache_reader.restore(root)
    }
}

#[cfg(test)]
mod test {
    use anyhow::Result;
    use futures::future::try_join_all;
    use tempfile::tempdir;
    use turbopath::AbsoluteSystemPathBuf;
    use turborepo_api_client::APIClient;
    use vercel_api_mock::start_test_server;

    use crate::{
        http::HttpCache,
        test_cases::{get_test_cases, TestCase},
        CacheOpts, CacheSource,
    };

    #[tokio::test]
    async fn test_http_cache() -> Result<()> {
        try_join_all(get_test_cases().into_iter().map(round_trip_test)).await?;

        Ok(())
    }

    async fn round_trip_test(test_case: TestCase) -> Result<()> {
        let port = port_scanner::request_open_port().unwrap();
        let handle = tokio::spawn(start_test_server(port));

        let repo_root = tempdir()?;
        let repo_root_path = AbsoluteSystemPathBuf::try_from(repo_root.path())?;
        test_case.initialize(&repo_root_path)?;

        let TestCase {
            hash,
            files,
            duration,
        } = test_case;

        let api_client = APIClient::new(&format!("http://localhost:{}", port), 200, "2.0.0", true)?;
        let opts = CacheOpts::default();
        let team_id = "my-team";
        let token = "my-token";

        let cache = HttpCache::new(api_client, opts, repo_root_path.to_owned(), team_id, token);

        cache
            .put(
                &repo_root_path,
                hash,
                files.iter().map(|f| f.path.clone()).collect(),
                duration,
                "",
            )
            .await?;

        let cache_response = cache.exists(hash, "", "", None, false).await?;

        assert_eq!(cache_response.time_saved, duration);
        assert_eq!(cache_response.source, CacheSource::Remote);

        let (cache_response, received_files) = cache.retrieve(hash, "", "", None, false).await?;
        assert_eq!(cache_response.time_saved, duration);

        for (test_file, received_file) in files.iter().zip(received_files) {
            assert_eq!(received_file, test_file.path);
            let file_path = repo_root_path.resolve(&received_file);
            assert_eq!(std::fs::read_to_string(file_path)?, test_file.contents);
        }

        handle.abort();
        Ok(())
    }
}
