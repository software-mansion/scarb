use anyhow::Result;

use scarb::core::Config;
use scarb::ops::{self, VersionControl};

use crate::args::NewArgs;

#[tracing::instrument(skip_all, level = "info")]
pub fn run(args: NewArgs, config: &Config) -> Result<()> {
    let result = ops::new_package(
        ops::InitOptions {
            name: args.init.name,
            path: args.path,
            // At the moment, we only support Git but ideally, we want to
            // support more VCS and allow user to explicitly specify which VCS to use.
            vcs: if args.init.no_vcs {
                VersionControl::NoVcs
            } else {
                VersionControl::Git
            },
            snforge: args.init.snforge,
        },
        config,
    )?;

    config
        .ui()
        .print(format!("Created `{}` package.", result.name));
    Ok(())
}
