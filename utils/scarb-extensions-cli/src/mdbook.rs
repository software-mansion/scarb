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
    /// Print machine-readable output in NDJSON format.
    #[arg(long, env = "SCARB_OUTPUT_JSON")]
    pub json: bool,
    /// Logging verbosity.
    #[command(flatten)]
    pub verbose: VerbositySpec,
}

impl Args {
    /// Construct [`scarb_ui::OutputFormat`] value from these arguments.
    pub fn output_format(&self) -> scarb_ui::OutputFormat {
        if self.json {
            scarb_ui::OutputFormat::Json
        } else {
            scarb_ui::OutputFormat::default()
        }
    }
}
