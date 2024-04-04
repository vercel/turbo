use std::{
    collections::HashSet,
    io::{ErrorKind, IsTerminal},
    sync::Arc,
    time::SystemTime,
};

use chrono::Local;
use rayon::iter::ParallelBridge;
use tracing::debug;
use turbopath::{AbsoluteSystemPathBuf, AnchoredSystemPath};
use turborepo_analytics::{start_analytics, AnalyticsHandle, AnalyticsSender};
use turborepo_api_client::{APIAuth, APIClient};
use turborepo_cache::{AsyncCache, RemoteCacheOpts};
use turborepo_env::EnvironmentVariableMap;
use turborepo_errors::Spanned;
use turborepo_repository::{
    package_graph::{PackageGraph, PackageName},
    package_json,
    package_json::PackageJson,
};
use turborepo_scm::SCM;
use turborepo_telemetry::events::{
    command::CommandEventBuilder,
    generic::{DaemonInitStatus, GenericEventBuilder},
    repo::{RepoEventBuilder, RepoType},
    EventBuilder, TrackedErrors,
};
use turborepo_ui::{ColorSelector, UI};
#[cfg(feature = "daemon-package-discovery")]
use {
    crate::run::package_discovery::DaemonPackageDiscovery,
    std::time::Duration,
    turborepo_repository::discovery::{
        Error as DiscoveryError, FallbackPackageDiscovery, LocalPackageDiscoveryBuilder,
        PackageDiscoveryBuilder,
    },
};

use crate::{
    cli::{DryRunMode, EnvMode},
    commands::CommandBase,
    engine::{Engine, EngineBuilder},
    opts::Opts,
    process::ProcessManager,
    run::{scope, task_access::TaskAccess, task_id::TaskName, Error, Run, RunCache},
    shim::TurboState,
    signal::{SignalHandler, SignalSubscriber},
    task_hash::PackageInputsHashes,
    turbo_json::TurboJson,
    DaemonConnector,
};

pub struct RunBuilder {
    processes: ProcessManager,
    opts: Opts,
    api_auth: Option<APIAuth>,
    repo_root: AbsoluteSystemPathBuf,
    ui: UI,
    version: &'static str,
    experimental_ui: bool,
    api_client: APIClient,
}

impl RunBuilder {
    pub fn new(base: CommandBase) -> Result<Self, Error> {
        let api_auth = base.api_auth()?;
        let api_client = base.api_client()?;

        let mut opts: Opts = base.args().try_into()?;
        let config = base.config()?;
        let is_linked = turborepo_api_client::is_linked(&api_auth);
        if !is_linked {
            opts.cache_opts.skip_remote = true;
        } else if let Some(enabled) = config.enabled {
            // We're linked, but if the user has explicitly enabled or disabled, use that
            // value
            opts.cache_opts.skip_remote = !enabled;
        }
        // Note that we don't currently use the team_id value here. In the future, we
        // should probably verify that we only use the signature value when the
        // configured team_id matches the final resolved team_id.
        let unused_remote_cache_opts_team_id = config.team_id().map(|team_id| team_id.to_string());
        let signature = config.signature();
        opts.cache_opts.remote_cache_opts = Some(RemoteCacheOpts::new(
            unused_remote_cache_opts_team_id,
            signature,
        ));
        if opts.run_opts.experimental_space_id.is_none() {
            opts.run_opts.experimental_space_id = config.spaces_id().map(|s| s.to_owned());
        }
        let version = base.version();
        let experimental_ui = config.experimental_ui();
        let processes = ProcessManager::new(
            // We currently only use a pty if the following are met:
            // - we're attached to a tty
            atty::is(atty::Stream::Stdout) &&
            // - if we're on windows, we're using the UI
            (!cfg!(windows) || experimental_ui),
        );
        let CommandBase { repo_root, ui, .. } = base;
        Ok(Self {
            processes,
            opts,
            api_client,
            api_auth,
            repo_root,
            ui,
            version,
            experimental_ui,
        })
    }

    fn connect_process_manager(&self, signal_subscriber: SignalSubscriber) {
        let manager = self.processes.clone();
        tokio::spawn(async move {
            let _guard = signal_subscriber.listen().await;
            manager.stop().await;
        });
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

    #[tracing::instrument(skip(self, signal_handler))]
    pub async fn build(
        mut self,
        signal_handler: &SignalHandler,
        telemetry: CommandEventBuilder,
    ) -> Result<Run, Error> {
        tracing::trace!(
            platform = %TurboState::platform_name(),
            start_time = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).expect("system time after epoch").as_micros(),
            turbo_version = %TurboState::version(),
            numcpus = num_cpus::get(),
            "performing run on {:?}",
            TurboState::platform_name(),
        );
        let start_at = Local::now();
        if let Some(subscriber) = signal_handler.subscribe() {
            self.connect_process_manager(subscriber);
        }

        let (analytics_sender, analytics_handle) =
            Self::initialize_analytics(self.api_auth.clone(), self.api_client.clone()).unzip();

        let scm = {
            let repo_root = self.repo_root.clone();
            tokio::task::spawn_blocking(move || SCM::new(&repo_root))
        };
        let package_json_path = self.repo_root.join_component("package.json");
        let root_package_json = PackageJson::load(&package_json_path)?;
        let run_telemetry = GenericEventBuilder::new().with_parent(&telemetry);
        let repo_telemetry =
            RepoEventBuilder::new(&self.repo_root.to_string()).with_parent(&telemetry);

        // Pulled from initAnalyticsClient in run.go
        let is_linked = turborepo_api_client::is_linked(&self.api_auth);
        run_telemetry.track_is_linked(is_linked);
        // we only track the remote cache if we're linked because this defaults to
        // Vercel
        if is_linked {
            run_telemetry.track_remote_cache(self.api_client.base_url());
        }
        let _is_structured_output = self.opts.run_opts.graph.is_some()
            || matches!(self.opts.run_opts.dry_run, Some(DryRunMode::Json));

        let is_single_package = self.opts.run_opts.single_package;
        repo_telemetry.track_type(if is_single_package {
            RepoType::SinglePackage
        } else {
            RepoType::Monorepo
        });

        let is_ci_or_not_tty = turborepo_ci::is_ci() || !std::io::stdout().is_terminal();
        run_telemetry.track_ci(turborepo_ci::Vendor::get_name());

        // Remove allow when daemon is flagged back on
        #[allow(unused_mut)]
        let mut daemon = match (is_ci_or_not_tty, self.opts.run_opts.daemon) {
            (true, None) => {
                run_telemetry.track_daemon_init(DaemonInitStatus::Skipped);
                debug!("skipping turbod since we appear to be in a non-interactive context");
                None
            }
            (_, Some(true)) | (false, None) => {
                let can_start_server = true;
                let can_kill_server = true;
                let connector =
                    DaemonConnector::new(can_start_server, can_kill_server, &self.repo_root);
                match (connector.connect().await, self.opts.run_opts.daemon) {
                    (Ok(client), _) => {
                        run_telemetry.track_daemon_init(DaemonInitStatus::Started);
                        debug!("running in daemon mode");
                        Some(client)
                    }
                    (Err(e), Some(true)) => {
                        run_telemetry.track_daemon_init(DaemonInitStatus::Failed);
                        debug!("failed to connect to daemon when forced {e}, exiting");
                        return Err(e.into());
                    }
                    (Err(e), None) => {
                        run_telemetry.track_daemon_init(DaemonInitStatus::Failed);
                        debug!("failed to connect to daemon {e}");
                        None
                    }
                    (_, Some(false)) => unreachable!(),
                }
            }
            (_, Some(false)) => {
                run_telemetry.track_daemon_init(DaemonInitStatus::Disabled);
                debug!("skipping turbod since --no-daemon was passed");
                None
            }
        };

        let mut pkg_dep_graph = {
            let builder = PackageGraph::builder(&self.repo_root, root_package_json.clone())
                .with_single_package_mode(self.opts.run_opts.single_package);

            #[cfg(feature = "daemon-package-discovery")]
            let graph = {
                match (&daemon, self.opts.run_opts.daemon) {
                    (None, Some(true)) => {
                        // We've asked for the daemon, but it's not available. This is an error
                        return Err(turborepo_repository::package_graph::Error::Discovery(
                            DiscoveryError::Unavailable,
                        )
                        .into());
                    }
                    (Some(daemon), Some(true)) => {
                        // We have the daemon, and have explicitly asked to only use that
                        let daemon_discovery = DaemonPackageDiscovery::new(daemon.clone());
                        builder
                            .with_package_discovery(daemon_discovery)
                            .build()
                            .await
                    }
                    (_, Some(false)) | (None, _) => {
                        // We have explicitly requested to not use the daemon, or we don't have it
                        // No change to default.
                        builder.build().await
                    }
                    (Some(daemon), None) => {
                        // We have the daemon, and it's not flagged off. Use the fallback strategy
                        let daemon_discovery = DaemonPackageDiscovery::new(daemon.clone());
                        let local_discovery = LocalPackageDiscoveryBuilder::new(
                            self.repo_root.clone(),
                            None,
                            Some(root_package_json.clone()),
                        )
                        .build()?;
                        let fallback_discover = FallbackPackageDiscovery::new(
                            daemon_discovery,
                            local_discovery,
                            Duration::from_millis(10),
                        );
                        builder
                            .with_package_discovery(fallback_discover)
                            .build()
                            .await
                    }
                }
            };
            #[cfg(not(feature = "daemon-package-discovery"))]
            let graph = builder.build().await;

            match graph {
                Ok(graph) => graph,
                // if we can't find the package.json, it is a bug, and we should report it.
                // likely cause is that package discovery watching is not up to date.
                // note: there _is_ a false positive from a race condition that can occur
                //       from toctou if the package.json is deleted, but we'd like to know
                Err(turborepo_repository::package_graph::Error::PackageJson(
                    package_json::Error::Io(io),
                )) if io.kind() == ErrorKind::NotFound => {
                    run_telemetry.track_error(TrackedErrors::InvalidPackageDiscovery);
                    return Err(turborepo_repository::package_graph::Error::PackageJson(
                        package_json::Error::Io(io),
                    )
                    .into());
                }
                Err(e) => return Err(e.into()),
            }
        };

        repo_telemetry.track_package_manager(pkg_dep_graph.package_manager().to_string());
        repo_telemetry.track_size(pkg_dep_graph.len());
        run_telemetry.track_run_type(self.opts.run_opts.dry_run.is_some());

        let scm = scm.await.expect("detecting scm panicked");
        let async_cache = AsyncCache::new(
            &self.opts.cache_opts,
            &self.repo_root,
            self.api_client.clone(),
            self.api_auth.clone(),
            analytics_sender,
        )?;

        // restore config from task access trace if it's enabled
        let task_access = TaskAccess::new(self.repo_root.clone(), async_cache.clone(), &scm);
        task_access.restore_config().await;

        let root_turbo_json = TurboJson::load(
            &self.repo_root,
            AnchoredSystemPath::empty(),
            &root_package_json,
            is_single_package,
        )?;
        root_turbo_json.track_usage(&run_telemetry);

        pkg_dep_graph.validate()?;

        let filtered_pkgs = {
            let (mut filtered_pkgs, is_all_packages) = scope::resolve_packages(
                &self.opts.scope_opts,
                &self.repo_root,
                &pkg_dep_graph,
                &scm,
                &root_turbo_json,
            )?;

            if is_all_packages {
                for target in self.opts.run_opts.tasks.iter() {
                    let mut task_name = TaskName::from(target.as_str());
                    // If it's not a package task, we convert to a root task
                    if !task_name.is_package_task() {
                        task_name = task_name.into_root_task()
                    }

                    if root_turbo_json.pipeline.contains_key(&task_name) {
                        filtered_pkgs.insert(PackageName::Root);
                        break;
                    }
                }
            };

            filtered_pkgs
        };

        let env_at_execution_start = EnvironmentVariableMap::infer();
        let mut engine = self.build_engine(&pkg_dep_graph, &root_turbo_json, &filtered_pkgs)?;

        let workspaces = pkg_dep_graph.packages().collect();
        let package_inputs_hashes = PackageInputsHashes::calculate_file_hashes(
            &scm,
            engine.tasks().par_bridge(),
            workspaces,
            engine.task_definitions(),
            &self.repo_root,
            &run_telemetry,
        )?;

        if self.opts.run_opts.parallel {
            pkg_dep_graph.remove_package_dependencies();
            engine = self.build_engine(&pkg_dep_graph, &root_turbo_json, &filtered_pkgs)?;
        }
        engine.track_usage(&run_telemetry);

        let color_selector = ColorSelector::default();

        let run_cache = Arc::new(RunCache::new(
            async_cache,
            &self.repo_root,
            &self.opts.runcache_opts,
            color_selector,
            daemon,
            self.ui,
            self.opts.run_opts.dry_run.is_some(),
        ));

        if matches!(self.opts.run_opts.env_mode, EnvMode::Infer)
            && root_turbo_json.global_pass_through_env.is_some()
        {
            self.opts.run_opts.env_mode = EnvMode::Strict;
        }

        Ok(Run {
            version: self.version,
            ui: self.ui,
            experimental_ui: self.experimental_ui,
            analytics_handle,
            start_at,
            processes: self.processes,
            run_telemetry,
            task_access,
            repo_root: self.repo_root,
            opts: self.opts,
            api_client: self.api_client,
            api_auth: self.api_auth,
            env_at_execution_start,
            filtered_pkgs,
            pkg_dep_graph: Arc::new(pkg_dep_graph),
            root_turbo_json,
            package_inputs_hashes,
            scm,
            engine: Arc::new(engine),
            run_cache,
            signal_handler: signal_handler.clone(),
        })
    }

    fn build_engine(
        &self,
        pkg_dep_graph: &PackageGraph,
        root_turbo_json: &TurboJson,
        filtered_pkgs: &HashSet<PackageName>,
    ) -> Result<Engine, Error> {
        let engine = EngineBuilder::new(
            &self.repo_root,
            pkg_dep_graph,
            self.opts.run_opts.single_package,
        )
        .with_root_tasks(root_turbo_json.pipeline.keys().cloned())
        .with_turbo_jsons(Some(
            Some((PackageName::Root, root_turbo_json.clone()))
                .into_iter()
                .collect(),
        ))
        .with_tasks_only(self.opts.run_opts.only)
        .with_workspaces(filtered_pkgs.clone().into_iter().collect())
        .with_tasks(self.opts.run_opts.tasks.iter().map(|task| {
            // TODO: Pull span info from command
            Spanned::new(TaskName::from(task.as_str()).into_owned())
        }))
        .build()?;

        if !self.opts.run_opts.parallel {
            engine
                .validate(
                    pkg_dep_graph,
                    self.opts.run_opts.concurrency,
                    self.experimental_ui,
                )
                .map_err(Error::EngineValidation)?;
        }

        Ok(engine)
    }
}
