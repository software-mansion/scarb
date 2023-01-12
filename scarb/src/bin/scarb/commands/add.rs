use anyhow::Result;
use scarb::core::Config;

#[tracing::instrument(skip_all, level = "info")]
pub fn run(_conf: &Config) -> Result<()> {
    todo!()
}
