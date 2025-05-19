#![deny(missing_docs)]

//! Extension CLI arguments datastructures.

use camino::Utf8PathBuf;
use clap::Parser;
use scarb_ui::args::VerbositySpec;

/// CLI command name.
pub const COMMAND_NAME: &str = "mdbook";

/// Build `mdBook` documentation
#[derive(Parser, Debug)]
#[command(name = COMMAND_NAME, version, about, long_about = None)]
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
