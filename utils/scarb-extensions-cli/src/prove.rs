use crate::execute::ExecutionArgs;
use clap::Parser;
use scarb_ui::args::{PackagesFilter, VerbositySpec};

/// Prove `scarb execute` output using Stwo prover.
#[derive(Parser, Clone, Debug)]
#[clap(version, verbatim_doc_comment)]
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

    #[command(flatten)]
    pub execute_args: ExecutionArgs,

    #[command(flatten)]
    pub prover: ProverArgs,

    /// Logging verbosity.
    #[command(flatten)]
    pub verbose: VerbositySpec,
}

#[derive(Parser, Clone, Debug)]
pub struct ProverArgs {
    /// Track relations during proving.
    #[arg(long, default_value = "false")]
    pub track_relations: bool,

    /// Display components during proving.
    #[arg(long, default_value = "false")]
    pub display_components: bool,
}
