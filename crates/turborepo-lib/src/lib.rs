#![feature(assert_matches)]
#![feature(box_patterns)]
#![feature(error_generic_member_access)]
#![feature(hash_extract_if)]
#![feature(option_get_or_insert_default)]
#![feature(once_cell_try)]
#![deny(clippy::all)]
// Clippy's needless mut lint is buggy: https://github.com/rust-lang/rust-clippy/issues/11299
#![allow(clippy::needless_pass_by_ref_mut)]
#![allow(dead_code)]

mod child;
mod cli;
mod commands;
mod config;
mod daemon;
mod engine;
mod execution_state;
mod framework;
pub(crate) mod globwatcher;
mod hash;
mod opts;
mod process;
mod rewrite_json;
mod run;
mod shim;
mod signal;
mod task_graph;
mod task_hash;
mod tracing;

pub use child::spawn_child;

use crate::commands::CommandBase;
pub use crate::{cli::Args, execution_state::ExecutionState};

/// The payload from running main, if the program can complete without using Go
/// the Rust variant will be returned. If Go is needed then the execution state
/// that should be passed to Go will be returned.
pub enum Payload {
    Rust(Result<i32, shim::Error>),
    Go(Box<CommandBase>),
}

pub fn get_version() -> &'static str {
    include_str!("../../../version.txt")
        .split_once('\n')
        .expect("Failed to read version from version.txt")
        .0
        // On windows we still have a trailing \r
        .trim_end()
}

pub fn main() -> Payload {
    match shim::run() {
        Ok(payload) => payload,
        // We don't need to print "Turbo error" for Run errors
        Err(err @ shim::Error::Cli(cli::Error::Run(_))) => Payload::Rust(Err(err)),
        Err(err) => {
            // This raw print matches the Go behavior, once we no longer care
            // about matching formatting we should remove this.
            println!("Turbo error: {err}");
            Payload::Rust(Err(err))
        }
    }
}
