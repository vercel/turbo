#![allow(dead_code)]

mod cache;
mod error;
pub(crate) mod global_hash;
mod scope;
pub(crate) mod summary;
pub mod task_id;

use std::{
    collections::HashSet,
    io::{BufWriter, IsTerminal, Write},
    sync::Arc,
    time::SystemTime,
};

pub use cache::{RunCache, TaskCache};
use chrono::{DateTime, Local};
use itertools::Itertools;
use rayon::iter::ParallelBridge;
use tracing::debug;
use turbopath::AbsoluteSystemPathBuf;
use turborepo_analytics::{start_analytics, AnalyticsHandle, AnalyticsSender};
use turborepo_api_client::{APIAuth, APIClient};
use turborepo_cache::{AsyncCache, RemoteCacheOpts};
use turborepo_ci::Vendor;
use turborepo_env::EnvironmentVariableMap;
use turborepo_repository::{
    package_graph::{PackageGraph, WorkspaceName},
    package_json::PackageJson,
};
use turborepo_scm::SCM;
use turborepo_ui::{cprint, cprintln, ColorSelector, BOLD_GREY, GREY};

use self::task_id::TaskName;
pub use crate::run::error::Error;
use crate::{
    cli::{DryRunMode, EnvMode},
    commands::CommandBase,
    config::TurboJson,
    daemon::DaemonConnector,
    engine::{Engine, EngineBuilder},
    opts::{GraphOpts, Opts},
    process::ProcessManager,
    run::{global_hash::get_global_hash_inputs, summary::RunTracker},
    shim::TurboState,
    signal::SignalSubscriber,
    task_graph::Visitor,
    task_hash::{get_external_deps_hash, PackageInputsHashes, TaskHashTrackerState},
};

#[derive(Debug)]
pub struct Run<'a> {
    base: &'a CommandBase,
    processes: ProcessManager,
}

impl<'a> Run<'a> {
    pub fn new(base: &'a CommandBase) -> Self {
        let processes = ProcessManager::new();
        Self { base, processes }
    }

    fn connect_process_manager(&self, signal_subscriber: SignalSubscriber) {
        let manager = self.processes.clone();
        tokio::spawn(async move {
            let _guard = signal_subscriber.listen().await;
            manager.stop().await;
        });
    }

    fn targets(&self) -> &[String] {
        self.base.args().get_tasks()
    }

    fn opts(&self) -> Result<Opts, Error> {
        Ok(self.base.args().try_into()?)
    }

    fn initialize_analytics(
        api_auth: Option<APIAuth>,
        api_client: APIClient,
    ) -> Option<(AnalyticsSender, AnalyticsHandle)> {
        // If there's no API auth, we don't want to record analytics
        let api_auth = api_auth?;
        api_auth
            .is_linked()
            .then(|| start_analytics(api_auth, api_client))
    }

    fn print_run_prelude(&self, opts: &Opts<'_>, filtered_pkgs: &HashSet<WorkspaceName>) {
        let targets_list = opts.run_opts.tasks.join(", ");
        if opts.run_opts.single_package {
            cprint!(self.base.ui, GREY, "{}", "• Running");
            cprint!(self.base.ui, BOLD_GREY, " {}\n", targets_list);
        } else {
            let mut packages = filtered_pkgs
                .iter()
                .map(|workspace_name| workspace_name.to_string())
                .collect::<Vec<String>>();
            packages.sort();
            cprintln!(
                self.base.ui,
                GREY,
                "• Packages in scope: {}",
                packages.join(", ")
            );
            cprint!(self.base.ui, GREY, "{} ", "• Running");
            cprint!(self.base.ui, BOLD_GREY, "{}", targets_list);
            cprint!(self.base.ui, GREY, " in {} packages\n", filtered_pkgs.len());
        }

        let use_http_cache = !opts.cache_opts.skip_remote;
        if use_http_cache {
            cprintln!(self.base.ui, GREY, "• Remote caching enabled");
        } else {
            cprintln!(self.base.ui, GREY, "• Remote caching disabled");
        }
    }

    #[tracing::instrument(skip(self, signal_subscriber))]
    pub async fn run(&mut self, signal_subscriber: SignalSubscriber) -> Result<i32, Error> {
        tracing::trace!(
            platform = %TurboState::platform_name(),
            start_time = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).expect("system time after epoch").as_micros(),
            turbo_version = %TurboState::version(),
            numcpus = num_cpus::get(),
            "performing run on {:?}",
            TurboState::platform_name(),
        );
        let start_at = Local::now();
        self.connect_process_manager(signal_subscriber);

        let api_auth = self.base.api_auth()?;
        let api_client = self.base.api_client()?;
        let (analytics_sender, analytics_handle) =
            Self::initialize_analytics(api_auth.clone(), api_client.clone()).unzip();

        let result = self
            .run_with_analytics(start_at, api_auth, api_client, analytics_sender)
            .await;

        if let Some(analytics_handle) = analytics_handle {
            analytics_handle.close_with_timeout().await;
        }

        result
    }

    // We split this into a separate function because we need
    // to close the AnalyticsHandle regardless of whether the run succeeds or not
    async fn run_with_analytics(
        &mut self,
        start_at: DateTime<Local>,
        api_auth: Option<APIAuth>,
        api_client: APIClient,
        analytics_sender: Option<AnalyticsSender>,
    ) -> Result<i32, Error> {
        let package_json_path = self.base.repo_root.join_component("package.json");
        let root_package_json = PackageJson::load(&package_json_path)?;
        let mut opts = self.opts()?;

        let config = self.base.config()?;

        // Pulled from initAnalyticsClient in run.go
        let is_linked = api_auth
            .as_ref()
            .map_or(false, |api_auth| api_auth.is_linked());
        if !is_linked {
            opts.cache_opts.skip_remote = true;
        } else if let Some(enabled) = config.enabled {
            // We're linked, but if the user has explicitly enabled or disabled, use that
            // value
            opts.cache_opts.skip_remote = !enabled;
        }

        let _is_structured_output = opts.run_opts.graph.is_some()
            || matches!(opts.run_opts.dry_run, Some(DryRunMode::Json));

        let is_single_package = opts.run_opts.single_package;

        let mut pkg_dep_graph =
            PackageGraph::builder(&self.base.repo_root, root_package_json.clone())
                .with_single_package_mode(opts.run_opts.single_package)
                .build()?;

        let root_turbo_json =
            TurboJson::load(&self.base.repo_root, &root_package_json, is_single_package)?;

        let team_id = root_turbo_json
            .remote_cache
            .as_ref()
            .and_then(|configuration_options| configuration_options.team_id.clone())
            .unwrap_or_default();

        let signature = root_turbo_json
            .remote_cache
            .as_ref()
            .and_then(|configuration_options| configuration_options.signature)
            .unwrap_or_default();

        opts.cache_opts.remote_cache_opts = Some(RemoteCacheOpts::new(team_id, signature));

        if opts.run_opts.experimental_space_id.is_none() {
            opts.run_opts.experimental_space_id = root_turbo_json.space_id.clone();
        }

        // There's some warning handling code in Go that I'm ignoring
        let is_ci_or_not_tty = turborepo_ci::is_ci() || !std::io::stdout().is_terminal();

        let daemon = if is_ci_or_not_tty && !opts.run_opts.no_daemon {
            debug!("skipping turbod since we appear to be in a non-interactive context");
            None
        } else if !opts.run_opts.no_daemon {
            let connector = DaemonConnector {
                can_start_server: true,
                can_kill_server: true,
                pid_file: self.base.daemon_file_root().join_component("turbod.pid"),
                sock_file: self.base.daemon_file_root().join_component("turbod.sock"),
            };

            match connector.connect().await {
                Ok(client) => {
                    debug!("running in daemon mode");
                    Some(client)
                }
                Err(e) => {
                    debug!("failed to connect to daemon {e}");
                    None
                }
            }
        } else {
            // We are opted out of using the daemon
            None
        };

        pkg_dep_graph.validate()?;

        let scm = SCM::new(&self.base.repo_root);

        let filtered_pkgs = {
            let mut filtered_pkgs = scope::resolve_packages(
                &opts.scope_opts,
                &self.base.repo_root,
                &pkg_dep_graph,
                &scm,
            )?;

            if filtered_pkgs.len() != pkg_dep_graph.len() {
                for target in self.targets() {
                    let mut task_name = TaskName::from(target.as_str());
                    // If it's not a package task, we convert to a root task
                    if !task_name.is_package_task() {
                        task_name = task_name.into_root_task()
                    }

                    if root_turbo_json.pipeline.contains_key(&task_name) {
                        filtered_pkgs.insert(WorkspaceName::Root);
                        break;
                    }
                }
            };

            filtered_pkgs
        };

        let env_at_execution_start = EnvironmentVariableMap::infer();

        let async_cache = AsyncCache::new(
            &opts.cache_opts,
            &self.base.repo_root,
            api_client.clone(),
            api_auth.clone(),
            analytics_sender,
        )?;

        let mut engine =
            self.build_engine(&pkg_dep_graph, &opts, &root_turbo_json, &filtered_pkgs)?;

        if opts.run_opts.dry_run.is_none() && opts.run_opts.graph.is_none() {
            self.print_run_prelude(&opts, &filtered_pkgs);
        }

        let root_workspace = pkg_dep_graph
            .workspace_info(&WorkspaceName::Root)
            .expect("must have root workspace");

        let is_monorepo = !opts.run_opts.single_package;

        let root_external_dependencies_hash =
            is_monorepo.then(|| get_external_deps_hash(&root_workspace.transitive_dependencies));

        let mut global_hash_inputs = get_global_hash_inputs(
            root_external_dependencies_hash.as_deref(),
            &self.base.repo_root,
            pkg_dep_graph.package_manager(),
            pkg_dep_graph.lockfile(),
            &root_turbo_json.global_deps,
            &env_at_execution_start,
            &root_turbo_json.global_env,
            root_turbo_json.global_pass_through_env.as_deref(),
            opts.run_opts.env_mode,
            opts.run_opts.framework_inference,
            root_turbo_json.global_dot_env.as_deref(),
        )?;

        let global_hash = global_hash_inputs.calculate_global_hash_from_inputs();

        debug!("global hash: {}", global_hash);

        let color_selector = ColorSelector::default();

        let runcache = Arc::new(RunCache::new(
            async_cache,
            &self.base.repo_root,
            &opts.runcache_opts,
            color_selector,
            daemon,
            self.base.ui,
            opts.run_opts.dry_run.is_some(),
        ));

        let mut global_env_mode = opts.run_opts.env_mode;
        if matches!(global_env_mode, EnvMode::Infer)
            && root_turbo_json.global_pass_through_env.is_some()
        {
            global_env_mode = EnvMode::Strict;
        }

        let workspaces = pkg_dep_graph.workspaces().collect();
        let package_inputs_hashes = PackageInputsHashes::calculate_file_hashes(
            &scm,
            engine.tasks().par_bridge(),
            workspaces,
            engine.task_definitions(),
            &self.base.repo_root,
        )?;

        if opts.run_opts.parallel {
            pkg_dep_graph.remove_workspace_dependencies();
            engine = self.build_engine(&pkg_dep_graph, &opts, &root_turbo_json, &filtered_pkgs)?;
        }

        if let Some(graph_opts) = opts.run_opts.graph {
            match graph_opts {
                GraphOpts::File(graph_file) => {
                    let graph_file =
                        AbsoluteSystemPathBuf::from_unknown(self.base.cwd(), graph_file);
                    let file = graph_file
                        .open()
                        .map_err(|e| Error::OpenGraphFile(e, graph_file.clone()))?;
                    let _writer = BufWriter::new(file);
                    todo!("Need to implement different format support");
                }
                GraphOpts::Stdout => {
                    engine
                        .dot_graph(std::io::stdout(), opts.run_opts.single_package)
                        .map_err(Error::GraphOutput)?;
                }
            }
            return Ok(0);
        }

        // remove dead code warnings
        let _proc_manager = ProcessManager::new();

        let pkg_dep_graph = Arc::new(pkg_dep_graph);
        let engine = Arc::new(engine);

        let global_env = {
            let mut env = env_at_execution_start
                .from_wildcards(global_hash_inputs.pass_through_env.unwrap_or_default())
                .map_err(Error::Env)?;
            if let Some(resolved_global) = &global_hash_inputs.resolved_env_vars {
                env.union(&resolved_global.all);
            }
            env
        };

        let run_tracker = RunTracker::new(
            start_at,
            opts.synthesize_command(),
            opts.scope_opts.pkg_inference_root.as_deref(),
            &env_at_execution_start,
            &self.base.repo_root,
            self.base.version(),
            opts.run_opts.experimental_space_id.clone(),
            api_client,
            api_auth,
            Vendor::get_user(),
        );

        let mut visitor = Visitor::new(
            pkg_dep_graph.clone(),
            runcache,
            run_tracker,
            &opts,
            package_inputs_hashes,
            &env_at_execution_start,
            &global_hash,
            global_env_mode,
            self.base.ui,
            false,
            self.processes.clone(),
            &self.base.repo_root,
            global_env,
        );

        if opts.run_opts.dry_run.is_some() {
            visitor.dry_run();
        }

        // we look for this log line to mark the start of the run
        // in benchmarks, so please don't remove it
        debug!("running visitor");

        let errors = visitor.visit(engine.clone()).await?;

        let exit_code = errors
            .iter()
            .filter_map(|err| err.exit_code())
            .max()
            // We hit some error, it shouldn't be exit code 0
            .unwrap_or(if errors.is_empty() { 0 } else { 1 });

        let error_prefix = if opts.run_opts.is_github_actions {
            "::error::"
        } else {
            ""
        };
        for err in &errors {
            writeln!(std::io::stderr(), "{error_prefix}{err}").ok();
        }

        visitor
            .finish(
                exit_code,
                filtered_pkgs,
                global_hash_inputs,
                &engine,
                &env_at_execution_start,
            )
            .await?;

        Ok(exit_code)
    }

    #[tokio::main]
    #[tracing::instrument(skip(self))]
    pub async fn get_hashes(&self) -> Result<(String, TaskHashTrackerState), Error> {
        let started_at = Local::now();
        let env_at_execution_start = EnvironmentVariableMap::infer();

        let package_json_path = self.base.repo_root.join_component("package.json");
        let root_package_json = PackageJson::load(&package_json_path)?;

        let opts = self.opts()?;

        let is_single_package = opts.run_opts.single_package;

        let mut pkg_dep_graph =
            PackageGraph::builder(&self.base.repo_root, root_package_json.clone())
                .with_single_package_mode(opts.run_opts.single_package)
                .build()?;

        let root_turbo_json =
            TurboJson::load(&self.base.repo_root, &root_package_json, is_single_package)?;

        let root_workspace = pkg_dep_graph
            .workspace_info(&WorkspaceName::Root)
            .expect("must have root workspace");

        let is_monorepo = !opts.run_opts.single_package;
        let root_external_dependencies_hash =
            is_monorepo.then(|| get_external_deps_hash(&root_workspace.transitive_dependencies));

        let mut global_hash_inputs = get_global_hash_inputs(
            root_external_dependencies_hash.as_deref(),
            &self.base.repo_root,
            pkg_dep_graph.package_manager(),
            pkg_dep_graph.lockfile(),
            &root_turbo_json.global_deps,
            &env_at_execution_start,
            &root_turbo_json.global_env,
            root_turbo_json.global_pass_through_env.as_deref(),
            opts.run_opts.env_mode,
            opts.run_opts.framework_inference,
            root_turbo_json.global_dot_env.as_deref(),
        )?;

        let scm = SCM::new(&self.base.repo_root);

        let filtered_pkgs = {
            let mut filtered_pkgs = scope::resolve_packages(
                &opts.scope_opts,
                &self.base.repo_root,
                &pkg_dep_graph,
                &scm,
            )?;

            if filtered_pkgs.len() != pkg_dep_graph.len() {
                for target in self.targets() {
                    let task_name = TaskName::from(target.as_str()).into_root_task();

                    if root_turbo_json.pipeline.contains_key(&task_name) {
                        filtered_pkgs.insert(WorkspaceName::Root);
                        break;
                    }
                }
            }

            filtered_pkgs
        };

        let global_hash = global_hash_inputs.calculate_global_hash_from_inputs();
        let api_auth = self.base.api_auth()?;

        let mut engine = EngineBuilder::new(
            &self.base.repo_root,
            &pkg_dep_graph,
            opts.run_opts.single_package,
        )
        .with_root_tasks(root_turbo_json.pipeline.keys().cloned())
        .with_turbo_jsons(Some(
            Some((WorkspaceName::Root, root_turbo_json.clone()))
                .into_iter()
                .collect(),
        ))
        .with_tasks_only(opts.run_opts.only)
        .with_workspaces(filtered_pkgs.clone().into_iter().collect())
        .with_tasks(
            opts.run_opts
                .tasks
                .iter()
                .map(|task| TaskName::from(task.as_str()).into_owned()),
        )
        .build()?;

        let mut global_env_mode = opts.run_opts.env_mode;
        if matches!(global_env_mode, EnvMode::Infer)
            && root_turbo_json.global_pass_through_env.is_some()
        {
            global_env_mode = EnvMode::Strict;
        }

        let package_inputs_hashes = PackageInputsHashes::calculate_file_hashes(
            &scm,
            engine.tasks().par_bridge(),
            pkg_dep_graph.workspaces().collect(),
            engine.task_definitions(),
            &self.base.repo_root,
        )?;

        if opts.run_opts.parallel {
            pkg_dep_graph.remove_workspace_dependencies();
            engine = self.build_engine(&pkg_dep_graph, &opts, &root_turbo_json, &filtered_pkgs)?;
        }

        let pkg_dep_graph = Arc::new(pkg_dep_graph);
        let engine = Arc::new(engine);
        let api_client = self.base.api_client()?;

        let async_cache = AsyncCache::new(
            &opts.cache_opts,
            &self.base.repo_root,
            api_client.clone(),
            api_auth.clone(),
            None,
        )?;

        let color_selector = ColorSelector::default();

        let runcache = Arc::new(RunCache::new(
            async_cache,
            &self.base.repo_root,
            &opts.runcache_opts,
            color_selector,
            None,
            self.base.ui,
            // Always dry run when getting hashes
            true,
        ));

        let run_tracker = RunTracker::new(
            started_at,
            opts.synthesize_command(),
            opts.scope_opts.pkg_inference_root.as_deref(),
            &env_at_execution_start,
            &self.base.repo_root,
            self.base.version(),
            opts.run_opts.experimental_space_id.clone(),
            api_client,
            api_auth,
            Vendor::get_user(),
        );

        let mut visitor = Visitor::new(
            pkg_dep_graph.clone(),
            runcache,
            run_tracker,
            &opts,
            package_inputs_hashes,
            &env_at_execution_start,
            &global_hash,
            global_env_mode,
            self.base.ui,
            true,
            self.processes.clone(),
            &self.base.repo_root,
            // TODO: this is only needed for full execution, figure out better way to model this
            // not affecting a dry run
            EnvironmentVariableMap::default(),
        );

        visitor.dry_run();

        visitor.visit(engine.clone()).await?;
        let task_hash_tracker = visitor.into_task_hash_tracker();

        Ok((global_hash, task_hash_tracker))
    }

    fn build_engine(
        &self,
        pkg_dep_graph: &PackageGraph,
        opts: &Opts,
        root_turbo_json: &TurboJson,
        filtered_pkgs: &HashSet<WorkspaceName>,
    ) -> Result<Engine, Error> {
        let engine = EngineBuilder::new(
            &self.base.repo_root,
            pkg_dep_graph,
            opts.run_opts.single_package,
        )
        .with_root_tasks(root_turbo_json.pipeline.keys().cloned())
        .with_turbo_jsons(Some(
            Some((WorkspaceName::Root, root_turbo_json.clone()))
                .into_iter()
                .collect(),
        ))
        .with_tasks_only(opts.run_opts.only)
        .with_workspaces(filtered_pkgs.clone().into_iter().collect())
        .with_tasks(
            opts.run_opts
                .tasks
                .iter()
                .map(|task| TaskName::from(task.as_str()).into_owned()),
        )
        .build()?;

        if !opts.run_opts.parallel {
            engine
                .validate(pkg_dep_graph, opts.run_opts.concurrency)
                .map_err(|errors| {
                    Error::EngineValidation(
                        errors
                            .into_iter()
                            .map(|e| e.to_string())
                            .sorted()
                            .join("\n"),
                    )
                })?;
        }

        Ok(engine)
    }
}
