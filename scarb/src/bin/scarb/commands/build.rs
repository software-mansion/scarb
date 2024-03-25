use anyhow::{anyhow, Result};

use crate::args::BuildArgs;
use scarb::core::{Config, TargetKind};
use scarb::ops;
use scarb::ops::CompileOpts;

#[tracing::instrument(skip_all, level = "info")]
pub fn run(args: BuildArgs, config: &Config) -> Result<()> {
    let ws = ops::read_workspace(config.manifest_path(), config)?;
    let packages = args
        .packages_filter
        .match_many(&ws)?
        .into_iter()
        .map(|p| p.id)
        .collect::<Vec<_>>();

    // TODO: support for multiple packages
    let package = args.packages_filter.match_many(&ws).unwrap()[0].clone();
    let features = package.manifest.features.clone().unwrap();
    
    if let Some(build_features_str) = args.features {
        for f in build_features_str.split(",").into_iter() {
            if !features.contains_key(f) {
                // TODO: maybe change error message
                return Err(anyhow!("Feature '{}' not found in .toml file", f));
            }
        }
    }

    let (include_targets, exclude_targets): (Vec<TargetKind>, Vec<TargetKind>) = if args.test {
        (vec![TargetKind::TEST.clone()], Vec::new())
    } else {
        (Vec::new(), vec![TargetKind::TEST.clone()])
    };
    let opts = CompileOpts {
        include_targets,
        exclude_targets,
    };
    ops::compile(packages, opts, &ws)
}
