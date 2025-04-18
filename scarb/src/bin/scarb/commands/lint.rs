use crate::args::LintArgs;
use anyhow::Result;
use scarb::core::Config;

#[tracing::instrument(skip_all, level = "info")]
pub fn run(args: LintArgs, config: &Config) -> Result<()> {
    do_lint(args, config)
}

#[cfg(feature = "scarb-lint")]
fn do_lint(args: LintArgs, config: &Config) -> Result<()> {
    use scarb::ops::{self, LintOptions};

    let ws = ops::read_workspace(config.manifest_path(), config)?;
    let packages = args
        .packages_filter
        .match_many(&ws)?
        .into_iter()
        .collect::<Vec<_>>();
    ops::lint(
        LintOptions {
            packages,
            target_names: args.target_names,
            test: args.test,
            fix: args.fix,
            ignore_cairo_version: args.ignore_cairo_version,
            features: args.features.try_into()?,
            deny_warnings: args.deny_warnings,
            load_prebuilt_macros: !args.no_prebuilt_proc_macros,
        },
        &ws,
    )
}

#[cfg(not(feature = "scarb-lint"))]
fn do_lint(_args: LintArgs, _config: &Config) -> Result<()> {
    anyhow::bail!("scarb was not compiled with the `lint` command enabled")
}
