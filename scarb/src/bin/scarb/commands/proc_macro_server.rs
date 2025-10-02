use anyhow::Result;
use scarb::{core::Config, ops};

#[tracing::instrument(skip_all, level = "info")]
pub fn run(config: &Config) -> Result<()> {
    ops::start_proc_macro_server(config)
}
