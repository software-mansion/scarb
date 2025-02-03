use anyhow::{Context, Result};
use cairo_lang_runner::Arg;
use cairo_lang_utils::bigint::BigUintAsHex;
use camino::Utf8PathBuf;
use clap::{arg, Parser, ValueEnum};
use num_bigint::BigInt;
use scarb_ui::args::{PackagesFilter, VerbositySpec};
use std::fs;

/// Compiles a Cairo project and runs a function marked `#[executable]`.
/// Exits with 1 if the compilation or run fails, otherwise 0.
#[derive(Parser, Clone, Debug)]
#[clap(version, verbatim_doc_comment)]
pub struct Args {
    /// Name of the package.
    #[command(flatten)]
    pub packages_filter: PackagesFilter,

    #[command(flatten)]
    pub execution: ExecutionArgs,

    /// Logging verbosity.
    #[command(flatten)]
    pub verbose: VerbositySpec,
}

#[derive(Parser, Clone, Debug)]
pub struct ExecutionArgs {
    /// Do not rebuild the package.
    #[arg(long, default_value_t = false)]
    pub no_build: bool,

    #[command(flatten)]
    pub run: RunArgs,
}

#[derive(Parser, Clone, Debug)]
pub struct RunArgs {
    #[command(flatten)]
    pub arguments: ProgramArguments,

    /// Desired execution output, either default Standard or CairoPie
    #[arg(long, default_value = "standard")]
    pub output: OutputFormat,

    /// Execution target.
    #[arg(long, default_value = "standalone")]
    pub target: ExecutionTarget,

    /// Whether to print the program outputs.
    #[arg(long, default_value_t = false)]
    pub print_program_output: bool,

    /// Whether to print detailed execution resources.
    #[arg(long, default_value_t = false)]
    pub print_resource_usage: bool,
}

#[derive(Parser, Debug, Clone)]
pub struct ProgramArguments {
    /// Serialized arguments to the executable function.
    #[arg(long, value_delimiter = ',')]
    pub arguments: Vec<BigInt>,

    /// Serialized arguments to the executable function from a file.
    #[arg(long, conflicts_with = "arguments")]
    pub arguments_file: Option<Utf8PathBuf>,
}

impl ProgramArguments {
    pub fn read_arguments(self) -> Result<Vec<Arg>> {
        if let Some(path) = self.arguments_file {
            let file = fs::File::open(&path).with_context(|| "reading arguments file failed")?;
            let as_vec: Vec<BigUintAsHex> = serde_json::from_reader(file)
                .with_context(|| "deserializing arguments file failed")?;
            Ok(as_vec
                .into_iter()
                .map(|v| Arg::Value(v.value.into()))
                .collect())
        } else {
            Ok(self
                .arguments
                .iter()
                .map(|v| Arg::Value(v.into()))
                .collect())
        }
    }
}

#[derive(ValueEnum, Clone, Debug)]
pub enum OutputFormat {
    CairoPie,
    Standard,
}
impl OutputFormat {
    pub fn is_standard(&self) -> bool {
        matches!(self, OutputFormat::Standard)
    }
    pub fn is_cairo_pie(&self) -> bool {
        matches!(self, OutputFormat::CairoPie)
    }
}

#[derive(ValueEnum, Clone, Debug)]
pub enum ExecutionTarget {
    Bootloader,
    Standalone,
}

impl ExecutionTarget {
    pub fn is_standalone(&self) -> bool {
        matches!(self, ExecutionTarget::Standalone)
    }
}
