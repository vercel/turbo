#![feature(min_specialization)]

pub mod command_line;
pub mod custom;
pub mod dotenv;
pub mod filter;

use std::{env, sync::Mutex};

use anyhow::Result;
use indexmap::IndexMap;
use turbo_tasks::primitives::OptionStringVc;

pub use self::{
    command_line::CommandLineProcessEnvVc, custom::CustomProcessEnvVc, dotenv::DotenvProcessEnvVc,
    filter::FilterProcessEnvVc,
};

#[turbo_tasks::value(transparent)]
pub struct EnvMap(#[turbo_tasks(trace_ignore)] IndexMap<String, String>);

#[turbo_tasks::value_impl]
impl EnvMapVc {
    #[turbo_tasks::function]
    pub fn empty() -> Self {
        EnvMap(IndexMap::new()).cell()
    }
}

#[turbo_tasks::value_impl]
impl ProcessEnv for EnvMap {
    #[turbo_tasks::function]
    async fn read_all(self_vc: EnvMapVc) -> Result<EnvMapVc> {
        Ok(self_vc)
    }

    #[turbo_tasks::function]
    async fn read(self_vc: EnvMapVc, name: &str) -> OptionStringVc {
        case_insensitive_read(self_vc, name)
    }
}

#[turbo_tasks::value_trait]
pub trait ProcessEnv {
    // TODO SECURITY: From security perspective it's not good that we read *all* env
    // vars into the cache. This might store secrects into the persistent cache
    // which we want to avoid.
    // Instead we should use only `read_prefix` to read all env vars with a specific
    // prefix.
    /// Reads all env variables into a Map
    fn read_all(&self) -> EnvMapVc;

    /// Reads a single env variable. Ignores casing.
    fn read(&self, name: &str) -> OptionStringVc {
        case_insensitive_read(self.read_all(), name)
    }
}

#[turbo_tasks::function]
pub async fn case_insensitive_read(map: EnvMapVc, name: &str) -> Result<OptionStringVc> {
    Ok(OptionStringVc::cell(
        to_uppercase_map(map)
            .await?
            .get(&name.to_uppercase())
            .cloned(),
    ))
}

#[turbo_tasks::function]
async fn to_uppercase_map(map: EnvMapVc) -> Result<EnvMapVc> {
    let map = &*map.await?;
    let mut new = IndexMap::with_capacity(map.len());
    for (k, v) in map {
        new.insert(k.to_uppercase(), v.clone());
    }
    Ok(EnvMapVc::cell(new))
}

pub static GLOBAL_ENV_LOCK: Mutex<()> = Mutex::new(());

pub fn register() {
    turbo_tasks::register();
    include!(concat!(env!("OUT_DIR"), "/register.rs"));
}
