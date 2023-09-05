use std::collections::{HashMap, HashSet};

use anyhow::Result;
use globwalk::WalkType;
use thiserror::Error;
use tracing::debug;
use turbopath::{AbsoluteSystemPath, RelativeUnixPathBuf};
use turborepo_env::{
    get_global_hashable_env_vars, BySource, DetailedMap, EnvironmentVariableMap,
    EnvironmentVariablePairs,
};
use turborepo_lockfiles::Lockfile;
use turborepo_scm::SCM;
use turborepo_ui::UI;

use crate::{cli::EnvMode, package_json::PackageJson, package_manager::PackageManager};

static DEFAULT_ENV_VARS: [&str; 1] = ["VERCEL_ANALYTICS_ID"];

#[derive(Debug, Error)]
enum GlobalHashError {}

#[derive(Default)]
pub struct GlobalHashableInputs {
    global_cache_key: &'static str,
    global_file_hash_map: HashMap<RelativeUnixPathBuf, String>,
    root_external_deps_hash: String,
    env: Vec<String>,
    // Only Option to allow #[derive(Default)]
    resolved_env_vars: Option<DetailedMap>,
    pass_through_env: Option<Vec<String>>,
    env_mode: EnvMode,
    framework_inference: bool,
    dot_env: Vec<RelativeUnixPathBuf>,
}

#[allow(clippy::too_many_arguments)]
pub fn get_global_hash_inputs<L: ?Sized + Lockfile>(
    _ui: &UI,
    root_path: &AbsoluteSystemPath,
    root_package_json: &PackageJson,
    package_manager: &PackageManager,
    lockfile: Option<&L>,
    global_file_dependencies: Vec<String>,
    env_at_execution_start: &EnvironmentVariableMap,
    global_env: Vec<String>,
    global_pass_through_env: Vec<String>,
    env_mode: EnvMode,
    framework_inference: bool,
    dot_env: Vec<RelativeUnixPathBuf>,
) -> Result<GlobalHashableInputs> {
    let global_hashable_env_vars =
        get_global_hashable_env_vars(env_at_execution_start, &global_env)?;

    debug!(
        "global hash env vars {:?}",
        global_hashable_env_vars.all.names()
    );

    let mut global_deps = HashSet::new();

    if !global_file_dependencies.is_empty() {
        let globs = package_manager.get_workspace_globs(root_path)?;

        let files = globwalk::globwalk(
            root_path,
            &global_file_dependencies,
            &globs.raw_exclusions,
            WalkType::All,
        )?;

        for file in files {
            global_deps.insert(file);
        }
    }

    if lockfile.is_none() {
        global_deps.insert(root_path.join_component("package.json"));
        let lockfile_path = root_path.join_component(package_manager.get_lockfile());
        if lockfile_path.exists() {
            global_deps.insert(lockfile_path);
        }
    }

    let hasher = SCM::new(root_path);

    let global_deps_paths = global_deps
        .iter()
        .map(|p| root_path.anchor(p).expect("path should be from root"))
        .collect::<Vec<_>>();

    let mut global_file_hash_map =
        hasher.get_hashes_for_files(root_path, &global_deps_paths, false)?;

    if !dot_env.is_empty() {
        let dot_env_object = hasher.hash_existing_of(root_path, &dot_env)?;

        for (key, value) in dot_env_object {
            global_file_hash_map.insert(key, value);
        }
    }

    Ok(GlobalHashableInputs {
        global_cache_key,
        global_file_hash_map,
        root_external_deps_hash: "todo".to_string(),
        env: global_env,
        resolved_env_vars: Some(global_hashable_env_vars),
        pass_through_env: Some(global_pass_through_env),
        env_mode,
        framework_inference,
        dot_env,
    })
}

impl GlobalHashableInputs {
    fn calculate_global_hash_from_inputs(mut self) -> u64 {
        match self.env_mode {
            EnvMode::Infer if self.pass_through_env.is_some() => {
                self.env_mode = EnvMode::Strict;
            }
            EnvMode::Loose => {
                self.pass_through_env = None;
            }
            // Collapse `None` and `Some([])` to `Some([])` in strict mode
            // to match Go behavior
            EnvMode::Strict if self.pass_through_env.is_none() => {
                self.pass_through_env = Some(Vec::new());
            }
            _ => {}
        }

        self.calculate_global_hash()
    }

    fn calculate_global_hash(self) -> u64 {
        let global_hashable = GlobalHashable {
            global_cache_key: self.global_cache_key.to_string(),
            global_file_hash_map: self.global_file_hash_map,
            root_external_deps_hash: self.root_external_deps_hash,
            env: self.env,
            resolved_env_vars: self
                .resolved_env_vars
                .map(|evm| evm.all.to_hashable())
                .unwrap_or_default(),
            pass_through_env: self.pass_through_env.unwrap_or_default(),
            env_mode: self.env_mode,
            framework_inference: self.framework_inference,
            dot_env: self.dot_env,
        };

        global_hashable.hash()
    }
}

fn hash_global(_hashable: GlobalHashable) -> Result<String, GlobalHashError> {
    todo!()
}
