#![deny(missing_docs)]

//! Extension CLI arguments datastructures.

use crate::execute::ExecutionArgs;
use clap::Parser;
use scarb_ui::args::{PackagesFilter, VerbositySpec};

/// CLI command name.
pub const COMMAND_NAME: &str = "prove";

/// Prove `scarb execute` output using Stwo prover.
#[derive(Parser, Clone, Debug)]
#[clap(name = COMMAND_NAME, version, verbatim_doc_comment)]
pub struct Args {
    /// Name of the package.
    #[command(flatten)]
    pub packages_filter: PackagesFilter,

    /// ID of `scarb execute` *standard* output for given package, for which to generate proof.
    #[arg(
        long,
        conflicts_with_all = [
            "execute",
            "no_build",
            "arguments",
            "arguments_file",
            "output",
            "target",
            "print_program_output",
            "print_resource_usage",
            "executable_name",
            "executable_function"
        ]
    )]
    pub execution_id: Option<usize>,

    /// Execute the program before proving.
    #[arg(
        long,
        default_value_t = false,
        required_unless_present = "execution_id"
    )]
    pub execute: bool,

    /// Specify execution arguments.
    #[command(flatten)]
    pub execute_args: ExecutionArgs,

    /// Logging verbosity.
    #[command(flatten)]
    pub verbose: VerbositySpec,
}
