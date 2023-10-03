use std::collections::HashMap;

use serde::Serialize;
use turbopath::RelativeUnixPathBuf;
use turborepo_env::{DetailedMap, EnvironmentVariableMap, EnvironmentVariablePairs};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
// Contains the environment variable inputs for the global hash
pub struct GlobalEnvConfiguration<'a> {
    pub env: &'a [String],
    pub pass_through_env: &'a [String],
}

// Contains the environment variables that impacted the global hash
#[derive(Debug, Serialize)]
pub struct GlobalEnvVarSummary<'a> {
    pub specified: GlobalEnvConfiguration<'a>,

    pub configured: EnvironmentVariablePairs,
    pub inferred: EnvironmentVariablePairs,
    pub pass_through: EnvironmentVariablePairs,
}

#[derive(Debug, Serialize)]
pub struct GlobalHashSummary<'a> {
    pub global_cache_key: &'static str,
    pub global_file_hash_map: HashMap<RelativeUnixPathBuf, String>,
    pub root_external_deps_hash: &'a str,
    pub dot_env: &'a [RelativeUnixPathBuf],
    pub env_vars: GlobalEnvVarSummary<'a>,
}

impl<'a> GlobalHashSummary<'a> {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        global_cache_key: &'static str,
        global_file_hash_map: HashMap<RelativeUnixPathBuf, String>,
        root_external_deps_hash: &'a str,
        global_env: &'a [String],
        global_pass_through_env: &'a [String],
        global_dot_env: &'a [RelativeUnixPathBuf],
        resolved_env_vars: DetailedMap,
        resolved_pass_through_env_vars: EnvironmentVariableMap,
    ) -> Self {
        Self {
            global_cache_key,
            global_file_hash_map,
            root_external_deps_hash,

            env_vars: GlobalEnvVarSummary {
                specified: GlobalEnvConfiguration {
                    env: global_env,
                    pass_through_env: global_pass_through_env,
                },
                configured: resolved_env_vars.by_source.explicit.to_secret_hashable(),
                inferred: resolved_env_vars.by_source.matching.to_secret_hashable(),
                pass_through: resolved_pass_through_env_vars.to_secret_hashable(),
            },

            dot_env: global_dot_env,
        }
    }
}
