use anyhow::{anyhow, Result};

use scarb::core::Config;
use scarb::ops;

use crate::args::TestArgs;

#[tracing::instrument(skip_all, level = "info")]
pub fn run(args: TestArgs, config: &Config) -> Result<()> {
    let ws = ops::read_workspace(config.manifest_path(), config)?;

    // TODO: support for multiple packages
    let package = args.packages_filter.match_many(&ws).unwrap()[0].clone();
    let available_features = package.manifest.features.clone().unwrap();
    let enabled_features = args
        .features
        .map(|x| x.split(',').map(|y| y.to_string()).collect::<Vec<String>>());

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

    args.packages_filter
        .match_many(&ws)?
        .iter()
        .try_for_each(|package| {
            ops::execute_test_subcommand(package, &args.args, &ws, enabled_features.clone())
                .map(|_| ())
        })
}
