use anyhow::{anyhow, Result};

use scarb::core::{Config, PackageName};
use scarb::manifest_editor::{EditManifestOptions, Op, RemoveDependency};
use scarb::{manifest_editor, ops};

use crate::args::RemoveArgs;

#[tracing::instrument(skip_all, level = "info")]
pub fn run(args: RemoveArgs, config: &mut Config) -> Result<()> {
    let ws = ops::read_workspace(config.manifest_path(), config)?;

    // TODO(mkaput): Extract more generic pattern for this. See `Packages` struct in Cargo.
    let package = match args.package {
        Some(name) => ws
            .members()
            .find(|pkg| pkg.id.name == name)
            .ok_or_else(|| anyhow!("package `{name}` not found in workspace `{ws}`"))?,
        None => ws.current_package()?.clone(),
    };

    manifest_editor::edit(
        package.manifest_path(),
        build_ops(args.packages),
        EditManifestOptions {
            config,
            dry_run: args.dry_run,
        },
    )?;

    if !args.dry_run {
        // Reload the workspace since we have changed dependencies
        let ws = ops::read_workspace(config.manifest_path(), config)?;

        // Only try to resolve packages if network is allowed, which would be probably required.
        if config.network_allowed() {
            let _ = ops::resolve_workspace(&ws)?;
        }
    }

    Ok(())
}

fn build_ops(packages: Vec<PackageName>) -> Vec<Box<dyn Op>> {
    packages
        .into_iter()
        .map(|dep| -> Box<dyn Op> { Box::new(RemoveDependency { dep }) })
        .collect()
}
