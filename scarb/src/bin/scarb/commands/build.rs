use std::collections::{HashSet, VecDeque};

use anyhow::{anyhow, Result};
use itertools::Itertools;

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
    let package = args.packages_filter.match_many(&ws).unwrap()[0].to_owned();
    let features = package.manifest.features.to_owned().unwrap_or_default();
    let available_features: HashSet<String> = features.keys().cloned().collect();
    let cli_features: HashSet<String> = args.features.into_iter().collect();

    let mut selected_features: HashSet<String> = if !args.no_default_features {
        cli_features
            .union(
                &features
                    .get("default")
                    .map(|f| HashSet::from_iter(f.iter().cloned()))
                    .unwrap_or_default(),
            )
            .cloned()
            .collect()
    } else {
        cli_features
    };

    // BFS set of features
    let mut queue = VecDeque::new();
    queue.extend(selected_features.clone().into_iter());

    while let Some(key) = queue.pop_front() {
        if let Some(neighbors) = features.get(&key) {
            for neighbor in neighbors.iter() {
                if !selected_features.contains(neighbor) {
                    selected_features.insert(neighbor.clone());
                    queue.push_back(neighbor.clone());
                }
            }
        }
    }

    let not_found_features = selected_features
        .difference(&available_features)
        .collect_vec();
    if !not_found_features.is_empty() {
        return Err(anyhow!(
            "Unknown features: {}",
            not_found_features.iter().join(", ")
        ));
    }

    let enabled_features = available_features
        .intersection(&selected_features)
        .cloned()
        .collect_vec();

    let (include_targets, exclude_targets): (Vec<TargetKind>, Vec<TargetKind>) = if args.test {
        (vec![TargetKind::TEST.clone()], Vec::new())
    } else {
        (Vec::new(), vec![TargetKind::TEST.clone()])
    };

    let opts = CompileOpts {
        include_targets,
        exclude_targets,
        enabled_features,
    };
    ops::compile(packages, opts, &ws)
}
