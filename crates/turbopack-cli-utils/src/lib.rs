#![feature(async_closure)]
#![feature(min_specialization)]
#![feature(round_char_boundary)]
#![feature(thread_id_value)]

pub mod issue;
pub mod raw_trace;
pub mod runtime_entry;
pub mod source_context;
pub mod tracing;

pub fn register() {
    turbo_tasks::register();
    turbo_tasks_fs::register();
    turbopack_core::register();
    include!(concat!(env!("OUT_DIR"), "/register.rs"));
}
