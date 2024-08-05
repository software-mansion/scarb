use anyhow::Result;

use scarb::core::Config;
use scarb::manifest_editor::{AddDependency, DepId, EditManifestOptions, Op};
use scarb::{manifest_editor, ops};

use crate::args::{AddArgs, AddSourceArgs};

#[tracing::instrument(skip_all, level = "info")]
pub fn run(args: AddArgs, config: &mut Config) -> Result<()> {
    let ws = ops::read_workspace(config.manifest_path(), config)?;

    let package = args.packages_filter.match_one(&ws)?;

    manifest_editor::edit(
        package.manifest_path(),
        build_ops(args.packages, args.source),
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

fn build_ops(packages: Vec<DepId>, source: AddSourceArgs) -> Vec<Box<dyn Op>> {
    let template = AddDependency {
        dep: DepId::unspecified(),
        path: source.path,
        git: source.git,
        branch: source.git_ref.branch,
        tag: source.git_ref.tag,
        rev: source.git_ref.rev,
    };

    if packages.is_empty() {
        vec![Box::new(template)]
    } else {
        packages
            .into_iter()
            .map(|dep| -> Box<dyn Op> {
                Box::new(AddDependency {
                    dep,
                    ..template.clone()
                })
            })
            .collect()
    }
}
