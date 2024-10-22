use std::collections::BTreeMap;

use anyhow::Result;
use camino::Utf8PathBuf;
use itertools::Itertools;
use serde::Serializer;

use scarb::core::{Config, PackageName};
use scarb::ops;
use scarb::ops::{validate_features, PackageOpts};
use scarb_ui::Message;

use crate::args::PackageArgs;

#[tracing::instrument(skip_all, level = "info")]
pub fn run(args: PackageArgs, config: &Config) -> Result<()> {
    let ws = ops::read_workspace(config.manifest_path(), config)?;
    let packages = args
        .packages_filter
        .match_many(&ws)?
        .into_iter()
        .collect_vec();

    let features_opts = args.features.try_into()?;
    validate_features(&packages, &features_opts)?;
    let opts = PackageOpts {
        // Disable dirty repository checks when printing package files.
        allow_dirty: args.list || args.shared_args.allow_dirty,
        verify: !args.shared_args.no_verify,
        check_metadata: !args.no_metadata,
        features: features_opts,
        ignore_cairo_version: args.ignore_cairo_version,
    };

    let packages = packages.into_iter().map(|p| p.id).collect_vec();

    if args.list {
        let result = ops::package_list(&packages, &opts, &ws)?;
        ws.config().ui().print(ListMessage(result));
    } else {
        ops::package(&packages, &opts, &ws)?;
    }

    Ok(())
}

struct ListMessage(BTreeMap<PackageName, Vec<Utf8PathBuf>>);

impl Message for ListMessage {
    fn print_text(self)
    where
        Self: Sized,
    {
        let mut first = true;
        let single = self.0.len() == 1;
        for (package, files) in self.0 {
            if !single {
                if !first {
                    println!();
                }
                println!("{package}:",);
            }

            for file in files {
                println!("{file}");
            }

            first = false;
        }
    }

    fn structured<S: Serializer>(self, _ser: S) -> Result<S::Ok, S::Error>
    where
        Self: Sized,
    {
        todo!("JSON output is not implemented yet.")
    }
}
