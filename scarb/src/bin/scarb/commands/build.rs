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
    let available_features = package.manifest.features.clone().unwrap(); // TODO: don't unwrap here
    let enabled_features = args
        .features
        .map(|x| x.split(",").map(|y| y.to_string()).collect::<Vec<String>>());

    let mut not_found_features: Vec<String> = Vec::new();
    if let Some(enabled_features_str) = enabled_features.as_ref() {
        for f in enabled_features_str.iter() {
            if !available_features.contains_key(f) {
                // TODO: maybe change error message
                not_found_features.push(format!("'{f}'"));
            }
        }
    }
    if !not_found_features.is_empty() {
        return Err(anyhow!(
            "Feature{} {} not found in .toml file",
            if not_found_features.len() > 1 {
                "s"
            } else {
                ""
            },
            not_found_features.join(", ")
        ));
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
    ops::compile(packages, opts, &ws, &enabled_features)
}
