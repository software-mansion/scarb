use camino::Utf8PathBuf;
use clap::Parser;
use scarb_ui::args::VerbositySpec;

/// Build `mdBook` documentation
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Args {
    /// Path to book source directory.
    #[arg(long)]
    pub input: Utf8PathBuf,
    /// Path to book output directory.
    #[arg(long)]
    pub output: Utf8PathBuf,
    /// Logging verbosity.
    #[command(flatten)]
    pub verbose: VerbositySpec,
}
