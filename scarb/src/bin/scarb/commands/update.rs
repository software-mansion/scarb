use anyhow::Result;

use scarb::core::Config;
use scarb::ops;
use scarb::ops::ResolveOpts;

#[tracing::instrument(skip_all, level = "info")]
pub fn run(config: &Config) -> Result<()> {
    let ws = ops::read_workspace(config.manifest_path(), config)?;
    let opts = ResolveOpts { update: true };
    ops::resolve_workspace_with_opts(&ws, &opts)?;
    Ok(())
}
