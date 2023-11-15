use std::collections::BTreeMap;

use anyhow::Result;
use camino::Utf8PathBuf;
use serde::Serializer;

use scarb::core::{Config, PackageName};
use scarb::ops;
use scarb::ops::PackageOpts;
use scarb_ui::Message;

use crate::args::PackageArgs;

#[tracing::instrument(skip_all, level = "info")]
pub fn run(args: PackageArgs, config: &Config) -> Result<()> {
    let ws = ops::read_workspace(config.manifest_path(), config)?;
    let packages = args
        .packages_filter
        .match_many(&ws)?
        .into_iter()
        .map(|p| p.id)
        .collect::<Vec<_>>();

    let opts = PackageOpts {
        // Disable dirty repository checks when printing package files.
        allow_dirty: args.list || args.allow_dirty,
        verify: !args.no_verify,
    };

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
