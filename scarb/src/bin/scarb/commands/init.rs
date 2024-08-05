use std::env;

use anyhow::{anyhow, Result};
use camino::Utf8PathBuf;

use scarb::core::Config;
use scarb::ops::{self, VersionControl};

use crate::args::{InitArgs, TestRunner};
use crate::interactive::get_or_ask_for_test_runner;

#[tracing::instrument(skip_all, level = "info")]
pub fn run(args: InitArgs, config: &Config) -> Result<()> {
    let path = Utf8PathBuf::from_path_buf(env::current_dir()?)
        .map_err(|path| anyhow!("path `{}` is not UTF-8 encoded", path.display()))?;

    ops::init_package(
        ops::InitOptions {
            name: args.name,
            path,
            // At the moment, we only support Git but ideally, we want to
            // support more VCS and allow user to explicitly specify which VCS to use.
            vcs: if args.no_vcs {
                VersionControl::NoVcs
            } else {
                VersionControl::Git
            },
            snforge: matches!(
                get_or_ask_for_test_runner(args.test_runner)?,
                TestRunner::StarknetFoundry
            ),
        },
        config,
    )?;
    config.ui().print("Created package.");
    Ok(())
}
