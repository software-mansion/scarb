use anyhow::Result;

use scarb::core::Config;
use scarb::ops::{self, VersionControl};

use crate::args::{NewArgs, TestRunner};
use crate::interactive::get_or_ask_for_test_runner;

#[tracing::instrument(skip_all, level = "info")]
pub fn run(args: NewArgs, config: &Config) -> Result<()> {
    let test_runner = if args.init.no_test {
        TestRunner::None
    } else {
        get_or_ask_for_test_runner(args.init.test_runner)?
    };

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
            snforge: matches!(test_runner, TestRunner::StarknetFoundry),
        },
        config,
    )?;

    config
        .ui()
        .print(format!("Created `{}` package.", result.name));
    Ok(())
}
