use anyhow::Result;
use murek::core::Config;

use murek::ops;

#[tracing::instrument(skip_all, level = "info")]
pub fn run(config: &Config) -> Result<()> {
    let ws = ops::read_workspace(&config.manifest_path, config)?;
    ops::compile(&ws, Box::new(print_diagnostic))
}

fn print_diagnostic(diag: String) {
    println!("{diag}");
}
