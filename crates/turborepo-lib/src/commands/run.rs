use anyhow::Result;
use tracing::{debug, error};

use crate::{commands::CommandBase, run::Run};

pub async fn run(base: CommandBase) -> Result<()> {
    let mut run = Run::new(base);
    debug!("using the experimental rust codepath");
    debug!("configured run struct: {:?}", run);

    match run.run().await {
        Ok(_code) => Ok(()),
        Err(err) => {
            error!("run failed: {}", err);
            Err(err)
        }
    }
}
