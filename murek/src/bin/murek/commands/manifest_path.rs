use anyhow::Result;
use murek::core::Config;

#[tracing::instrument(skip_all, level = "info")]
pub fn run(config: &Config) -> Result<()> {
    println!("{}", config.manifest_path.display());
    Ok(())
}
