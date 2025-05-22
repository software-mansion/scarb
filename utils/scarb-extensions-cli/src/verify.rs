#![deny(missing_docs)]

//! Extension CLI arguments datastructures.

use camino::Utf8PathBuf;
use clap::Parser;
use scarb_ui::args::{PackagesFilter, VerbositySpec};

/// CLI command name.
pub const COMMAND_NAME: &str = "verify";

/// Verify `scarb prove` output using Stwo verifier
#[derive(Parser, Clone, Debug)]
#[clap(name = COMMAND_NAME, version, verbatim_doc_comment)]
pub struct Args {
    /// Name of the package.
    #[command(flatten)]
    pub packages_filter: PackagesFilter,

    /// ID of `scarb execute` output for given package, for which proof was generated using `scarb prove`.
    #[arg(long)]
    pub execution_id: Option<u32>,

    /// Proof file path.
    #[arg(
        long,
        required_unless_present = "execution_id",
        conflicts_with = "execution_id"
    )]
    pub proof_file: Option<Utf8PathBuf>,

    /// Logging verbosity.
    #[command(flatten)]
    pub verbose: VerbositySpec,
}
