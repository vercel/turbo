#![feature(future_join)]
#![feature(min_specialization)]

use std::path::Path;

use anyhow::{Context, Result};
use clap::Parser;
use once_cell::sync::Lazy;
use tracing_appender::non_blocking::DEFAULT_BUFFERED_LINES_LIMIT;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter, Registry};
use turbopack_cli::{arguments::Arguments, register};
use turbopack_cli_utils::{exit::exit_guard, raw_trace::RawTraceLayer};

#[global_allocator]
static ALLOC: turbo_tasks_malloc::TurboMalloc = turbo_tasks_malloc::TurboMalloc;

static TRACING_OVERVIEW_TARGETS: Lazy<Vec<&str>> =
    Lazy::new(|| vec!["turbo_tasks_fs=info", "turbopack_dev_server=info"]);
static TRACING_TURBOPACK_TARGETS: Lazy<Vec<&str>> = Lazy::new(|| {
    [
        &TRACING_OVERVIEW_TARGETS[..],
        &[
            "turbopack=trace",
            "turbopack_core=trace",
            "turbopack_ecmascript=trace",
            "turbopack_css=trace",
            "turbopack_dev=trace",
            "turbopack_image=trace",
            "turbopack_dev_server=trace",
            "turbopack_json=trace",
            "turbopack_mdx=trace",
            "turbopack_node=trace",
            "turbopack_static=trace",
            "turbopack_cli_utils=trace",
            "turbopack_cli=trace",
            "turbopack_ecmascript=trace",
        ],
    ]
    .concat()
});
static TRACING_TURBO_TASKS_TARGETS: Lazy<Vec<&str>> = Lazy::new(|| {
    [
        &TRACING_TURBOPACK_TARGETS[..],
        &[
            "turbo_tasks=trace",
            "turbo_tasks_viz=trace",
            "turbo_tasks_memory=trace",
            "turbo_tasks_fs=trace",
        ],
    ]
    .concat()
});

fn main() {
    use turbo_tasks_malloc::TurboMalloc;

    let args = Arguments::parse();

    let trace = std::env::var("TURBOPACK_TRACING").ok();

    let _guard = if let Some(mut trace) = trace {
        // Trace presets
        match trace.as_str() {
            "overview" => {
                trace = TRACING_OVERVIEW_TARGETS.join(",");
            }
            "turbopack" => {
                trace = TRACING_TURBOPACK_TARGETS.join(",");
            }
            "turbo-tasks" => {
                trace = TRACING_TURBO_TASKS_TARGETS.join(",");
            }
            _ => {}
        }

        let subscriber = Registry::default();

        let subscriber = subscriber.with(EnvFilter::builder().parse(trace).unwrap());

        let internal_dir = args
            .dir()
            .unwrap_or_else(|| Path::new("."))
            .join(".turbopack");
        std::fs::create_dir_all(&internal_dir)
            .context("Unable to create .turbopack directory")
            .unwrap();
        let trace_file = internal_dir.join("trace.log");
        let (writer, guard) = tracing_appender::non_blocking::NonBlockingBuilder::default()
            .lossy(false)
            .buffered_lines_limit(DEFAULT_BUFFERED_LINES_LIMIT * 8)
            .finish(std::fs::File::create(trace_file).unwrap());
        let subscriber = subscriber.with(RawTraceLayer::new(writer));

        let guard = exit_guard(guard).unwrap();

        subscriber.init();

        Some(guard)
    } else {
        None
    };

    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .on_thread_stop(|| {
            TurboMalloc::thread_stop();
        })
        .build()
        .unwrap()
        .block_on(main_inner(args))
        .unwrap();
}

async fn main_inner(args: Arguments) -> Result<()> {
    register();

    match args {
        Arguments::Dev(args) => turbopack_cli::dev::start_server(&args).await,
    }
}
