use anyhow::{Context, Result, ensure};
use cairo_lang_runner::Arg;
use cairo_lang_utils::bigint::BigUintAsHex;
use cairo_vm::Felt252;
use camino::Utf8PathBuf;
use clap::{Parser, ValueEnum, arg};
use scarb_ui::args::{FeaturesSpec, PackagesFilter, VerbositySpec};
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

    /// Specifies features to enable.
    #[command(flatten)]
    pub features: FeaturesSpec,

    #[command(flatten)]
    pub build_target_args: BuildTargetSpecifier,

    #[command(flatten)]
    pub run: RunArgs,
}

#[derive(Parser, Clone, Debug)]
pub struct BuildTargetSpecifier {
    /// Choose build target to run by target name.
    #[arg(long)]
    pub executable_name: Option<String>,

    /// Choose build target to run by function path.
    #[arg(long, conflicts_with = "executable_name")]
    pub executable_function: Option<String>,
}

#[derive(Parser, Clone, Debug)]
pub struct RunArgs {
    #[command(flatten)]
    pub arguments: ProgramArguments,

    /// Desired execution output, either default Standard or CairoPie
    #[arg(long)]
    pub output: Option<OutputFormat>,

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
    pub arguments: Vec<Felt252>,

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
            Ok(self.arguments.into_iter().map(Arg::Value).collect())
        }
    }
}

#[derive(ValueEnum, Clone, Debug)]
pub enum OutputFormat {
    CairoPie,
    Standard,
}
impl OutputFormat {
    pub fn default_for_target(target: ExecutionTarget) -> OutputFormat {
        match target {
            ExecutionTarget::Bootloader => OutputFormat::CairoPie,
            ExecutionTarget::Standalone => OutputFormat::Standard,
        }
    }
    pub fn validate(&self, target: &ExecutionTarget) -> Result<()> {
        ensure!(
            !(self.is_cairo_pie() && target.is_standalone()),
            "Cairo pie output format is not supported for standalone execution target"
        );
        ensure!(
            !(self.is_standard() && target.is_bootloader()),
            "Standard output format is not supported for bootloader execution target"
        );
        Ok(())
    }
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
    pub fn is_bootloader(&self) -> bool {
        matches!(self, ExecutionTarget::Bootloader)
    }
}
