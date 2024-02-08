use std::sync::Arc;

use reqwest::Url;
use tokio::sync::OnceCell;
use tracing::warn;
use turborepo_api_client::Client;
use turborepo_ui::{start_spinner, BOLD};

use crate::{auth::extract_vercel_token, error, ui, Error, LoginOptions, Token};

const DEFAULT_HOST_NAME: &str = "127.0.0.1";
const DEFAULT_PORT: u16 = 9789;
const DEFAULT_SSO_PROVIDER: &str = "SAML/OIDC Single Sign-On";

fn make_token_name() -> Result<String, Error> {
    let host = hostname::get().map_err(Error::FailedToMakeSSOTokenName)?;

    Ok(format!(
        "Turbo CLI on {} via {DEFAULT_SSO_PROVIDER}",
        host.to_string_lossy()
    ))
}

/// Perform an SSO login flow. If an existing token is present, and the token
/// has access to the provided `sso_team`, we do not overwrite it and instead
/// log that we found an existing token.
pub async fn sso_login<'a, T: Client>(options: &LoginOptions<'_, T>) -> Result<Token, Error> {
    let LoginOptions {
        api_client,
        ui,
        login_url: login_url_configuration,
        login_server,
        sso_team,
        existing_token: _,
    } = *options;

    let sso_team = sso_team.ok_or(Error::EmptySSOTeam)?;
    // Check if token exists first. Must be there for the user and contain the
    // sso_team passed into this function.
    if let Some(token) = options.existing_token {
        let (result_user, result_teams) =
            tokio::join!(api_client.get_user(token), api_client.get_teams(token));

        if let (Ok(response_user), Ok(response_teams)) = (result_user, result_teams) {
            if response_teams
                .teams
                .iter()
                .any(|team| team.slug == sso_team)
            {
                println!("{}", ui.apply(BOLD.apply_to("Existing token found!")));
                ui::print_cli_authorized(&response_user.user.email, ui);
                return Ok(Token::Existing(token.into()));
            }
        }
    }

    // No existing token found. If the user is logging into Vercel, check for an
    // existing `vc` token with correct scope.
    if login_url_configuration.contains("vercel.com") {
        match extract_vercel_token() {
            Ok(token) => {
                println!(
                    "{}",
                    ui.apply(BOLD.apply_to("Existing Vercel token found!"))
                );
                return Ok(Token::Existing(token));
            }
            Err(e) => {
                dbg!("Failed to extract Vercel token: ", e);
            }
        }
    }

    let redirect_url = format!("http://{DEFAULT_HOST_NAME}:{DEFAULT_PORT}");
    let mut login_url = Url::parse(login_url_configuration)?;

    login_url
        .path_segments_mut()
        .map_err(|_: ()| error::Error::LoginUrlCannotBeABase {
            value: login_url_configuration.to_string(),
        })?
        .extend(["api", "auth", "sso"]);

    login_url
        .query_pairs_mut()
        .append_pair("teamId", sso_team)
        .append_pair("mode", "login")
        .append_pair("next", &redirect_url);

    println!(">>> Opening browser to {login_url}");
    let spinner = start_spinner("Waiting for your authorization...");
    let url = login_url.as_str();

    // Don't open the browser in tests.
    if !cfg!(test) && webbrowser::open(url).is_err() {
        warn!("Failed to open browser. Please visit {url} in your browser.");
    }

    let token_cell = Arc::new(OnceCell::new());
    login_server
        .run(DEFAULT_PORT, crate::LoginType::SSO, token_cell.clone())
        .await?;
    spinner.finish_and_clear();

    let token = token_cell.get().ok_or(Error::FailedToGetToken)?;

    let token_name = make_token_name()?;

    let verified_user = api_client
        .verify_sso_token(token, &token_name)
        .await
        .map_err(Error::FailedToValidateSSOToken)?;

    let user_response = api_client
        .get_user(&verified_user.token)
        .await
        .map_err(Error::FailedToFetchUser)?;

    ui::print_cli_authorized(&user_response.user.email, ui);

    Ok(Token::New(verified_user.token))
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::AtomicUsize;

    use async_trait::async_trait;
    use reqwest::{Method, RequestBuilder, Response};
    use turborepo_api_client::Client;
    use turborepo_ui::UI;
    use turborepo_vercel_api::{
        CachingStatusResponse, Membership, Role, SpacesResponse, Team, TeamsResponse, User,
        UserResponse, VerifiedSsoUser,
    };
    use turborepo_vercel_api_mock::start_test_server;

    use super::*;
    use crate::{LoginServer, LoginType};
    const EXPECTED_VERIFICATION_TOKEN: &str = "expected_verification_token";

    lazy_static::lazy_static! {
        static ref SSO_HITS: Arc<AtomicUsize> = Arc::new(AtomicUsize::new(0));
    }

    #[derive(Debug, thiserror::Error)]
    enum MockApiError {
        #[error("Empty token")]
        EmptyToken,
    }

    impl From<MockApiError> for turborepo_api_client::Error {
        fn from(error: MockApiError) -> Self {
            match error {
                MockApiError::EmptyToken => turborepo_api_client::Error::UnknownStatus {
                    code: "empty token".to_string(),
                    message: "token is empty".to_string(),
                    backtrace: std::backtrace::Backtrace::capture(),
                },
            }
        }
    }

    struct MockApiClient {
        pub base_url: String,
    }

    impl MockApiClient {
        fn new() -> Self {
            Self {
                base_url: String::new(),
            }
        }

        fn set_base_url(&mut self, base_url: &str) {
            self.base_url = base_url.to_string();
        }
    }

    #[async_trait]
    impl Client for MockApiClient {
        async fn get_user(&self, token: &str) -> turborepo_api_client::Result<UserResponse> {
            if token.is_empty() {
                return Err(MockApiError::EmptyToken.into());
            }

            Ok(UserResponse {
                user: User {
                    id: "id".to_string(),
                    username: "username".to_string(),
                    email: "email".to_string(),
                    name: None,
                    created_at: None,
                },
            })
        }
        async fn get_teams(&self, token: &str) -> turborepo_api_client::Result<TeamsResponse> {
            if token.is_empty() {
                return Err(MockApiError::EmptyToken.into());
            }

            Ok(TeamsResponse {
                teams: vec![Team {
                    id: "id".to_string(),
                    slug: "something".to_string(),
                    name: "name".to_string(),
                    created_at: 0,
                    created: chrono::Utc::now(),
                    membership: Membership::new(Role::Member),
                }],
            })
        }
        async fn get_team(
            &self,
            _token: &str,
            _team_id: &str,
        ) -> turborepo_api_client::Result<Option<Team>> {
            unimplemented!("get_team")
        }
        fn add_ci_header(_request_builder: RequestBuilder) -> RequestBuilder {
            unimplemented!("add_ci_header")
        }
        async fn get_caching_status(
            &self,
            _token: &str,
            _team_id: Option<&str>,
            _team_slug: Option<&str>,
        ) -> turborepo_api_client::Result<CachingStatusResponse> {
            unimplemented!("get_caching_status")
        }
        async fn get_spaces(
            &self,
            _token: &str,
            _team_id: Option<&str>,
        ) -> turborepo_api_client::Result<SpacesResponse> {
            unimplemented!("get_spaces")
        }
        async fn verify_sso_token(
            &self,
            token: &str,
            _: &str,
        ) -> turborepo_api_client::Result<VerifiedSsoUser> {
            Ok(VerifiedSsoUser {
                token: token.to_string(),
                team_id: Some("team_id".to_string()),
            })
        }
        async fn put_artifact(
            &self,
            _hash: &str,
            _artifact_body: &[u8],
            _duration: u64,
            _tag: Option<&str>,
            _token: &str,
            _team_id: Option<&str>,
            _team_slug: Option<&str>,
        ) -> turborepo_api_client::Result<()> {
            unimplemented!("put_artifact")
        }
        async fn handle_403(_response: Response) -> turborepo_api_client::Error {
            unimplemented!("handle_403")
        }
        async fn fetch_artifact(
            &self,
            _hash: &str,
            _token: &str,
            _team_id: Option<&str>,
            _team_slug: Option<&str>,
        ) -> turborepo_api_client::Result<Option<Response>> {
            unimplemented!("fetch_artifact")
        }
        async fn artifact_exists(
            &self,
            _hash: &str,
            _token: &str,
            _team_id: Option<&str>,
            _team_slug: Option<&str>,
        ) -> turborepo_api_client::Result<Option<Response>> {
            unimplemented!("artifact_exists")
        }
        async fn get_artifact(
            &self,
            _hash: &str,
            _token: &str,
            _team_id: Option<&str>,
            _team_slug: Option<&str>,
            _method: Method,
        ) -> turborepo_api_client::Result<Option<Response>> {
            unimplemented!("get_artifact")
        }
        fn make_url(&self, endpoint: &str) -> turborepo_api_client::Result<Url> {
            let url = format!("{}{}", self.base_url, endpoint);
            Url::parse(&url).map_err(|err| turborepo_api_client::Error::InvalidUrl { url, err })
        }
    }

    #[derive(Clone)]
    struct MockSSOLoginServer {
        hits: Arc<AtomicUsize>,
    }

    #[async_trait]
    impl LoginServer for MockSSOLoginServer {
        async fn run(
            &self,
            _port: u16,
            _login_type: LoginType,
            login_token: Arc<OnceCell<String>>,
        ) -> Result<(), Error> {
            self.hits.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            login_token
                .set(EXPECTED_VERIFICATION_TOKEN.to_string())
                .unwrap();
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_sso_login() {
        let port = port_scanner::request_open_port().unwrap();
        let handle = tokio::spawn(start_test_server(port));
        let url = format!("http://localhost:{port}");
        let ui = UI::new(false);
        let team = "something";

        let mut api_client = MockApiClient::new();
        api_client.set_base_url(&url);

        let login_server = MockSSOLoginServer {
            hits: Arc::new(0.into()),
        };
        let mut options = LoginOptions {
            sso_team: Some(team),
            ..LoginOptions::new(&ui, &url, &api_client, &login_server)
        };

        let token = sso_login(&options).await.unwrap();
        assert!(!matches!(token, Token::Existing(..)));

        let got_token = token.into_inner().to_string();
        assert_eq!(got_token, EXPECTED_VERIFICATION_TOKEN.to_owned());

        // Call the login function twice to test that we check for existing tokens.
        // Total server hits should be 1.
        options.existing_token = Some(&got_token);
        let second_token = sso_login(&options).await.unwrap();
        assert!(matches!(second_token, Token::Existing(..)));

        handle.abort();

        // This makes sure we never make it to the login server.
        assert_eq!(
            login_server.hits.load(std::sync::atomic::Ordering::SeqCst),
            1
        );
    }
}
