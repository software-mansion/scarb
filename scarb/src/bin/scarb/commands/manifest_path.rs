use anyhow::Result;

use scarb::core::Config;

#[tracing::instrument(skip_all, level = "info")]
pub fn run(config: &Config) -> Result<()> {
    let canonical =
        dunce::canonicalize(&config.manifest_path).unwrap_or_else(|_| config.manifest_path.clone());
    println!("{}", canonical.display());
    Ok(())
}
