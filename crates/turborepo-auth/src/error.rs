use std::io;

use turbopath::AbsoluteSystemPathBuf;
use turborepo_api_client::Error as APIError;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    UrlParseError(#[from] url::ParseError),
    #[error(transparent)]
    APIError(#[from] APIError),
    #[error(transparent)]
    IOError(#[from] io::Error),

    #[error("failed to get token")]
    FailedToGetToken,

    #[error("failed to fetch user: {0}")]
    FailedToFetchUser(#[source] turborepo_api_client::Error),
    #[error("failed to fetch token metadata: {source}")]
    FailedToFetchTokenMetadata { source: turborepo_api_client::Error },
    #[error(
        "loginUrl is configured to \"{value}\", but cannot be a base URL. This happens in \
         situations like using a `data:` URL."
    )]
    LoginUrlCannotBeABase { value: String },

    // SSO errors
    #[error("failed to make sso token name")]
    FailedToMakeSSOTokenName(io::Error),
    #[error("failed to validate sso token")]
    FailedToValidateSSOToken(turborepo_api_client::Error),

    // File write errors
    #[error("failed to write to auth file at {auth_path}: {error}")]
    FailedToWriteAuth {
        auth_path: turbopath::AbsoluteSystemPathBuf,
        error: io::Error,
    },
    #[error("failed to create auth file at {auth_path}: {error}")]
    FailedToCreateAuthFile {
        auth_path: turbopath::AbsoluteSystemPathBuf,
        error: io::Error,
    },

    // File read errors.
    #[error(transparent)]
    PathError(#[from] turbopath::PathError),
    #[error("failed to read auth file at path: {path}")]
    FailedToReadAuthFile {
        #[source]
        source: std::io::Error,
        path: AbsoluteSystemPathBuf,
    },
    #[error("failed to read config file at path: {path}")]
    FailedToReadConfigFile {
        #[source]
        source: std::io::Error,
        path: AbsoluteSystemPathBuf,
    },

    // File write errors.
    #[error("failed to write AuthFile to disk: {source}")]
    FailedToWriteAuthFile {
        #[source]
        source: std::io::Error,
    },

    // Conversion errors.
    #[error("failed to deserialize auth file: {source}")]
    FailedToDeserializeAuthFile {
        #[source]
        source: serde_json::Error,
    },
    #[error("failed to deserialize config token: {source}")]
    FailedToDeserializeConfigToken {
        #[source]
        source: serde_json::Error,
    },
    #[error("failed to serialize auth file: {source}")]
    FailedToSerializeAuthFile {
        #[source]
        source: serde_json::Error,
    },

    #[error("failed to convert config to auth file: {source}")]
    ConvertConfigToAuth {
        #[source]
        source: serde_json::Error,
    },
}
