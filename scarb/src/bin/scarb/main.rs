use anyhow::Result;
use clap::Parser;
use tracing_log::AsTrace;
use tracing_subscriber::EnvFilter;

use args::ScarbArgs;
use scarb::core::Config;
use scarb::dirs::AppDirs;
use scarb::ops;
use scarb::ui::Ui;

mod args;
mod commands;

fn main() {
    let args = ScarbArgs::parse();

    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_env_filter(
            EnvFilter::builder()
                .with_default_directive(args.verbose.log_level_filter().as_trace().into())
                .with_env_var("SCARB_LOG")
                .from_env_lossy(),
        )
        .init();

    // Pre-create Ui used in error reporting, because we will move `args` to `cli_main`.
    let ui = Ui::new(args.ui_verbosity(), args.output_format());

    if let Err(err) = cli_main(args) {
        ui.anyhow(&err);
        std::process::exit(1);
    }
}

fn cli_main(args: ScarbArgs) -> Result<()> {
    let mut dirs = AppDirs::std()?;
    dirs.apply_env_overrides()?;

    let ui = Ui::new(args.ui_verbosity(), args.output_format());

    let manifest_path = ops::find_manifest_path(args.manifest_path.as_deref())?;
    let mut config = Config::init(manifest_path, dirs, ui, args.target_dir)?;
    config.set_offline(args.offline);
    commands::run(args.command, &mut config)
}
