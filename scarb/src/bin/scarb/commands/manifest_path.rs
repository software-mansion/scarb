use anyhow::Result;

use scarb::core::Config;
use scarb_ui::components::ValueMessage;

#[tracing::instrument(skip_all, level = "info")]
pub fn run(config: &Config) -> Result<()> {
    let canonical = dunce::canonicalize(config.manifest_path())
        .unwrap_or_else(|_| config.manifest_path().into());
    let canonical = canonical.to_string_lossy();
    config.ui().print(ValueMessage::new("path", &canonical));
    Ok(())
}
