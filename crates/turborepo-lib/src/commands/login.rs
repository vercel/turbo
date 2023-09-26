#[cfg(not(test))]
use std::net::SocketAddr;
use std::sync::Arc;

use anyhow::{anyhow, Context, Result};
#[cfg(not(test))]
use axum::{extract::Query, response::Redirect, routing::get, Router};
use reqwest::Url;
use serde::Deserialize;
use tokio::sync::OnceCell;
#[cfg(not(test))]
use tracing::warn;
use turborepo_api_client::APIClient;
use turborepo_ui::{start_spinner, BOLD, CYAN};

use crate::{commands::CommandBase, config::Error};

const DEFAULT_HOST_NAME: &str = "127.0.0.1";
const DEFAULT_PORT: u16 = 9789;
const DEFAULT_SSO_PROVIDER: &str = "SAML/OIDC Single Sign-On";

use turborepo_auth::login as auth_login;

pub async fn sso_login(base: &mut CommandBase, sso_team: &str) -> Result<()> {
    let repo_config = base.repo_config()?;
    let redirect_url = format!("http://{DEFAULT_HOST_NAME}:{DEFAULT_PORT}");
    let login_url_configuration = repo_config.login_url();
    let mut login_url = Url::parse(login_url_configuration)?;

    login_url
        .path_segments_mut()
        .map_err(|_: ()| Error::LoginUrlCannotBeABase {
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
    direct_user_to_url(login_url.as_str());

    let token_cell = Arc::new(OnceCell::new());
    run_sso_one_shot_server(DEFAULT_PORT, token_cell.clone()).await?;
    spinner.finish_and_clear();

    let token = token_cell
        .get()
        .ok_or_else(|| anyhow!("no token auth token found"))?;

    let token_name = make_token_name().context("failed to make sso token name")?;

    let api_client = base.api_client()?;
    let verified_user = api_client.verify_sso_token(token, &token_name).await?;
    let user_response = api_client.get_user(&verified_user.token).await?;

    base.user_config_mut()?
        .set_token(Some(verified_user.token.clone()))?;

    println!(
        "
{} {}
",
        base.ui.rainbow(">>> Success!"),
        base.ui.apply(BOLD.apply_to(format!(
            "Turborepo CLI authorized for {}",
            user_response.user.email
        )))
    );

    println!(
        "{}
{}
",
        base.ui.apply(
            CYAN.apply_to("To connect to your Remote Cache, run the following in any turborepo:")
        ),
        base.ui.apply(BOLD.apply_to("`npx turbo link`"))
    );

    Ok(())
}

fn make_token_name() -> Result<String> {
    let host = hostname::get()?;

    Ok(format!(
        "Turbo CLI on {} via {DEFAULT_SSO_PROVIDER}",
        host.to_string_lossy()
    ))
}

pub async fn login(base: &mut CommandBase) -> Result<()> {
    let api_client: APIClient = base.api_client()?;
    let ui = base.ui;
    let login_url_config = base.repo_config()?.login_url().to_string();

    // We are passing a closure here, but it would be cleaner if we made a
    // turborepo-config crate and imported that into turborepo-auth.
    let set_token = |token: &str| -> Result<(), anyhow::Error> {
        Ok(base.user_config_mut()?.set_token(Some(token.to_string()))?)
    };

    auth_login(api_client, &ui, set_token, &login_url_config).await
}

#[cfg(test)]
fn direct_user_to_url(_: &str) {}
#[cfg(not(test))]
fn direct_user_to_url(url: &str) {
    if webbrowser::open(url).is_err() {
        warn!("Failed to open browser. Please visit {url} in your browser.");
    }
}

#[cfg(test)]
const EXPECTED_VERIFICATION_TOKEN: &str = "expected_verification_token";

#[derive(Debug, Default, Clone, Deserialize)]
#[allow(dead_code)]
struct SsoPayload {
    login_error: Option<String>,
    sso_email: Option<String>,
    team_name: Option<String>,
    sso_type: Option<String>,
    token: Option<String>,
    email: Option<String>,
}

fn get_token_and_redirect(payload: SsoPayload) -> Result<(Option<String>, Url)> {
    let location_stub = "https://vercel.com/notifications/cli-login/turbo/";
    if let Some(login_error) = payload.login_error {
        let mut url = Url::parse(&format!("{}failed", location_stub))?;
        url.query_pairs_mut()
            .append_pair("loginError", login_error.as_str());
        return Ok((None, url));
    }

    if let Some(sso_email) = payload.sso_email {
        let mut url = Url::parse(&format!("{}incomplete", location_stub))?;
        url.query_pairs_mut()
            .append_pair("ssoEmail", sso_email.as_str());
        if let Some(team_name) = payload.team_name {
            url.query_pairs_mut()
                .append_pair("teamName", team_name.as_str());
        }
        if let Some(sso_type) = payload.sso_type {
            url.query_pairs_mut()
                .append_pair("ssoType", sso_type.as_str());
        }

        return Ok((None, url));
    }
    let mut url = Url::parse(&format!("{}success", location_stub))?;
    if let Some(email) = payload.email {
        url.query_pairs_mut().append_pair("email", email.as_str());
    }

    Ok((payload.token, url))
}

#[cfg(test)]
async fn run_sso_one_shot_server(_: u16, verification_token: Arc<OnceCell<String>>) -> Result<()> {
    verification_token
        .set(EXPECTED_VERIFICATION_TOKEN.to_string())
        .unwrap();
    Ok(())
}

#[cfg(not(test))]
async fn run_sso_one_shot_server(
    port: u16,
    verification_token: Arc<OnceCell<String>>,
) -> Result<()> {
    let handle = axum_server::Handle::new();
    let route_handle = handle.clone();
    let app = Router::new()
        // `GET /` goes to `root`
        .route(
            "/",
            get(|sso_payload: Query<SsoPayload>| async move {
                let (token, location) = get_token_and_redirect(sso_payload.0).unwrap();
                if let Some(token) = token {
                    // If token is already set, it's not a big deal, so we ignore the error.
                    let _ = verification_token.set(token);
                }
                route_handle.shutdown();
                Redirect::to(location.as_str())
            }),
        );
    let addr = SocketAddr::from(([127, 0, 0, 1], port));

    Ok(axum_server::bind(addr)
        .handle(handle)
        .serve(app.into_make_service())
        .await?)
}

#[cfg(test)]
mod test {
    use std::{cell::OnceCell, fs};

    use reqwest::Url;
    use serde::Deserialize;
    use tempfile::{tempdir, NamedTempFile};
    use turbopath::AbsoluteSystemPathBuf;
    use turborepo_ui::UI;
    use turborepo_vercel_api_mock::start_test_server;

    use crate::{
        commands::{
            login,
            login::{get_token_and_redirect, SsoPayload},
            CommandBase,
        },
        config::{ClientConfigLoader, RepoConfigLoader, UserConfigLoader},
        Args,
    };

    #[tokio::test]
    async fn test_login() {
        let port = port_scanner::request_open_port().unwrap();
        let handle = tokio::spawn(start_test_server(port));

        let user_config_file = NamedTempFile::new().unwrap();
        fs::write(user_config_file.path(), r#"{ "token": "hello" }"#).unwrap();
        let repo_config_file = NamedTempFile::new().unwrap();
        let repo_config_path = AbsoluteSystemPathBuf::try_from(repo_config_file.path()).unwrap();
        // Explicitly pass the wrong port to confirm that we're reading it from the
        // manual override
        fs::write(
            repo_config_file.path(),
            format!("{{ \"apiurl\": \"http://localhost:{}\" }}", port + 1),
        )
        .unwrap();
        let root_dir = tempdir().unwrap();

        let mut base = CommandBase {
            repo_root: AbsoluteSystemPathBuf::new(root_dir.path().to_string_lossy()).unwrap(),
            ui: UI::new(false),
            client_config: OnceCell::from(ClientConfigLoader::new().load().unwrap()),
            user_config: OnceCell::from(
                UserConfigLoader::new(user_config_file.path().to_str().unwrap())
                    .load()
                    .unwrap(),
            ),
            repo_config: OnceCell::from(
                RepoConfigLoader::new(repo_config_path)
                    .with_api(Some(format!("http://localhost:{}", port)))
                    .load()
                    .unwrap(),
            ),
            args: Args::default(),
            version: "",
        };

        login::login(&mut base).await.unwrap();

        handle.abort();

        assert_eq!(
            base.user_config().unwrap().token().unwrap(),
            turborepo_vercel_api_mock::EXPECTED_TOKEN
        );
    }

    #[derive(Debug, Clone, Deserialize)]
    struct TokenRequest {
        #[cfg(not(test))]
        redirect_uri: String,
    }

    #[tokio::test]
    async fn test_sso_login() {
        let port = port_scanner::request_open_port().unwrap();
        let handle = tokio::spawn(start_test_server(port));

        let user_config_file = NamedTempFile::new().unwrap();
        fs::write(user_config_file.path(), r#"{ "token": "hello" }"#).unwrap();
        let repo_config_file = NamedTempFile::new().unwrap();
        let repo_config_path = AbsoluteSystemPathBuf::try_from(repo_config_file.path()).unwrap();
        // Explicitly pass the wrong port to confirm that we're reading it from the
        // manual override
        fs::write(
            repo_config_file.path(),
            format!("{{ \"apiurl\": \"http://localhost:{}\" }}", port + 1),
        )
        .unwrap();

        let repo_root_dir = tempdir().unwrap();
        let mut base = CommandBase {
            repo_root: AbsoluteSystemPathBuf::new(repo_root_dir.path().to_string_lossy()).unwrap(),
            ui: UI::new(false),
            client_config: OnceCell::from(ClientConfigLoader::new().load().unwrap()),
            user_config: OnceCell::from(
                UserConfigLoader::new(user_config_file.path().to_str().unwrap())
                    .load()
                    .unwrap(),
            ),
            repo_config: OnceCell::from(
                RepoConfigLoader::new(repo_config_path)
                    .with_api(Some(format!("http://localhost:{}", port)))
                    .load()
                    .unwrap(),
            ),
            args: Args::default(),
            version: "",
        };

        login::sso_login(&mut base, turborepo_vercel_api_mock::EXPECTED_SSO_TEAM_SLUG)
            .await
            .unwrap();

        handle.abort();

        assert_eq!(
            base.user_config().unwrap().token().unwrap(),
            turborepo_vercel_api_mock::EXPECTED_TOKEN
        );
    }

    #[test]
    fn test_get_token_and_redirect() {
        assert_eq!(
            get_token_and_redirect(SsoPayload::default()).unwrap(),
            (
                None,
                Url::parse("https://vercel.com/notifications/cli-login/turbo/success").unwrap()
            )
        );

        assert_eq!(
            get_token_and_redirect(SsoPayload {
                login_error: Some("error".to_string()),
                ..SsoPayload::default()
            })
            .unwrap(),
            (
                None,
                Url::parse(
                    "https://vercel.com/notifications/cli-login/turbo/failed?loginError=error"
                )
                .unwrap()
            )
        );

        assert_eq!(
            get_token_and_redirect(SsoPayload {
                sso_email: Some("email".to_string()),
                ..SsoPayload::default()
            })
            .unwrap(),
            (
                None,
                Url::parse(
                    "https://vercel.com/notifications/cli-login/turbo/incomplete?ssoEmail=email"
                )
                .unwrap()
            )
        );

        assert_eq!(
            get_token_and_redirect(SsoPayload {
                sso_email: Some("email".to_string()),
                team_name: Some("team".to_string()),
                ..SsoPayload::default()
            }).unwrap(),
            (
                None,
                Url::parse("https://vercel.com/notifications/cli-login/turbo/incomplete?ssoEmail=email&teamName=team")
                    .unwrap()
            )
        );

        assert_eq!(
            get_token_and_redirect(SsoPayload {
                token: Some("token".to_string()),
                ..SsoPayload::default()
            })
            .unwrap(),
            (
                Some("token".to_string()),
                Url::parse("https://vercel.com/notifications/cli-login/turbo/success").unwrap()
            )
        );
    }
}
