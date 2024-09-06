use turborepo_ci::Vendor;
use turborepo_ui::{cprint, cprintln, ColorConfig, GREY, UNDERLINE, YELLOW};

use crate::EnvironmentVariableMap;

pub struct CIEnv {
    ci_keys: Vec<String>,
}

const SYSTEM_ENV_KEY: &str = "TURBO_SYSTEM_ENV";
const VALIDATE_PLATFORM_ENV: &str = "TURBO_VALIDATE_PLATFORM_ENV";

impl CIEnv {
    pub fn new() -> Self {
        let ci_keys = std::env::var(SYSTEM_ENV_KEY)
            .unwrap_or_default()
            .split(',')
            .map(|s| s.to_string())
            .collect();

        Self { ci_keys }
    }

    pub fn enabled() -> bool {
        let validate_platform_env = std::env::var(VALIDATE_PLATFORM_ENV).unwrap_or_default();
        validate_platform_env == "1" || validate_platform_env == "true"
    }

    pub fn validate(
        &self,
        execution_env: &EnvironmentVariableMap,
        color_config: ColorConfig,
        task_id_for_display: &str,
    ) {
        if !Self::enabled() {
            return;
        }

        let missing_env = self.diff(execution_env);
        if !missing_env.is_empty() {
            self.output(missing_env, task_id_for_display, color_config);
        }
    }

    pub fn diff(&self, execution_env: &EnvironmentVariableMap) -> Vec<String> {
        self.ci_keys
            .iter()
            .filter(|key| !execution_env.contains_key(*key))
            .map(|s| s.to_string())
            .collect()
    }

    pub fn output(
        &self,
        missing: Vec<String>,
        task_id_for_display: &str,
        color_config: ColorConfig,
    ) {
        let ci = Vendor::get_constant().unwrap_or("unknown");
        match ci {
            "VERCEL" => {
                cprintln!(color_config, YELLOW, "\n{} WARNINGS:", task_id_for_display);
                cprintln!(
                    color_config,
                    GREY,
                    "1. The following environment variables are set on your Vercel project, but \
                     missing from \"turbo.json\""
                );
                for key in missing {
                    cprint!(color_config, GREY, "  - ");
                    cprint!(color_config, UNDERLINE, "{}\n", key);
                }
                println!();
            }
            _ => {
                cprintln!(color_config, YELLOW, "\n{} WARNINGS:", task_id_for_display);
                cprintln!(
                    color_config,
                    GREY,
                    "1. The following environment variables are missing from \"turbo.json\""
                );
                for key in missing {
                    cprint!(color_config, GREY, "  - ");
                    cprint!(color_config, UNDERLINE, "{}\n", key);
                }
                println!();
            }
        }
    }
}
