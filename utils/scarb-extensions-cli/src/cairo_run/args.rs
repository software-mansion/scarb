use crate::cairo_run::deserialization;
use camino::Utf8PathBuf;
use clap::Parser;
use scarb_ui::args::PackagesFilter;
use scarb_ui::args::VerbositySpec;

/// Execute the main function of a package.
#[derive(Parser, Clone, Debug)]
#[command(author, version)]
pub struct Args {
    /// Name of the package.
    #[command(flatten)]
    pub packages_filter: PackagesFilter,

    /// Specify name of the function to run.
    #[arg(long)]
    pub function: Option<String>,

    /// Maximum amount of gas available to the program.
    #[arg(long)]
    pub available_gas: Option<usize>,

    /// Print more items in memory.
    #[arg(long, default_value_t = false)]
    pub print_full_memory: bool,

    /// Print detailed resources.
    #[arg(long, default_value_t = false)]
    pub print_resource_usage: bool,

    /// Do not rebuild the package.
    #[arg(long, default_value_t = false)]
    pub no_build: bool,

    /// Logging verbosity.
    #[command(flatten)]
    pub verbose: VerbositySpec,

    /// Program arguments.
    ///
    /// This should be a JSON array of numbers, decimal bigints or recursive arrays of those. For example, pass `[1]`
    /// to the following function `fn main(a: u64)`, or pass `[1, "2"]` to `fn main(a: u64, b: u64)`,
    /// or `[1, 2, [3, 4, 5]]` to `fn main(t: (u64, u64), v: Array<u64>)`.
    #[arg(default_value = "[]")]
    pub arguments: deserialization::Args,

    /// Path to the JSON file containing program arguments.
    ///
    /// It specified, `[ARGUMENTS]` CLI parameter will be ignored.
    #[arg(long)]
    pub arguments_file: Option<Utf8PathBuf>,
}
