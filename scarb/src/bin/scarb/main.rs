use anyhow::Result;
use clap::Parser;
use tracing_log::AsTrace;
use tracing_subscriber::fmt::format::FmtSpan;
use tracing_subscriber::EnvFilter;

use args::Args;
use scarb::core::Config;
use scarb::dirs::AppDirs;
use scarb::ops;

mod args;
mod commands;

fn main() -> Result<()> {
    let args: Args = Args::parse();

    tracing_subscriber::fmt()
        .with_span_events(FmtSpan::NEW | FmtSpan::CLOSE)
        .with_env_filter(
            EnvFilter::builder()
                .with_default_directive(args.verbose.log_level_filter().as_trace().into())
                .with_env_var("SCARB_LOG")
                .from_env_lossy(),
        )
        .init();

    let mut dirs = AppDirs::std()?;
    dirs.apply_env_overrides()?;

    let manifest_path = ops::find_manifest_path(args.manifest_path.as_deref())?;
    let mut config = Config::init(manifest_path, dirs)?;
    commands::run(args.command, &mut config)
}
