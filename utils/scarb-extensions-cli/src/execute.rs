#![deny(missing_docs)]

//! Extension CLI arguments datastructures.

use anyhow::{Result, ensure};
use cairo_vm::Felt252;
use camino::Utf8PathBuf;
use clap::{Parser, ValueEnum};
use scarb_ui::args::{FeaturesSpec, PackagesFilter, VerbositySpec};
use std::fmt::Display;

/// CLI command name.
pub const COMMAND_NAME: &str = "execute";

/// Compile a Cairo project and run a function marked `#[executable]`
#[derive(Parser, Clone, Debug)]
#[clap(name = COMMAND_NAME, version, verbatim_doc_comment)]
pub struct Args {
    /// Name of the package.
    #[command(flatten)]
    pub packages_filter: PackagesFilter,

    /// Specify execution arguments.
    #[command(flatten)]
    pub execution: ExecutionArgs,

    /// Logging verbosity.
    #[command(flatten)]
    pub verbose: VerbositySpec,
}

/// Execution arguments.
#[derive(Parser, Clone, Debug)]
pub struct ExecutionArgs {
    /// Do not rebuild the package.
    #[arg(long, default_value_t = false)]
    pub no_build: bool,

    /// Specifies features to enable.
    #[command(flatten)]
    pub features: FeaturesSpec,

    /// Choose build target to run.
    #[command(flatten)]
    pub build_target_args: BuildTargetSpecifier,

    /// Specify runner arguments.
    #[command(flatten)]
    pub run: RunArgs,
}

impl ToArgs for ExecutionArgs {
    fn to_args(&self) -> Vec<String> {
        let Self {
            no_build,
            build_target_args,
            run,
            // Should be passed via env.
            features: _,
        } = self;
        let mut args = Vec::new();
        if *no_build {
            args.push("--no-build".to_string());
        }
        args.extend(build_target_args.to_args());
        args.extend(run.to_args());
        args
    }
}

/// Build target specifier.
#[derive(Parser, Clone, Debug)]
pub struct BuildTargetSpecifier {
    /// Choose build target to run by target name.
    #[arg(long)]
    pub executable_name: Option<String>,

    /// Choose build target to run by function path.
    #[arg(long, conflicts_with = "executable_name")]
    pub executable_function: Option<String>,
}

impl ToArgs for BuildTargetSpecifier {
    fn to_args(&self) -> Vec<String> {
        let Self {
            executable_name,
            executable_function,
        } = self;
        if let Some(executable_name) = executable_name {
            return vec!["--executable-name".to_string(), executable_name.to_string()];
        }
        if let Some(executable_function) = executable_function {
            return vec![
                "--executable-function".to_string(),
                executable_function.to_string(),
            ];
        }
        Vec::new()
    }
}

/// Runner arguments.
#[derive(Parser, Clone, Debug)]
pub struct RunArgs {
    /// Pass arguments to the executable function.
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

    /// Enable experimental oracles support.
    #[arg(long, default_value_t = false, env = "SCARB_EXPERIMENTAL_ORACLES")]
    pub experimental_oracles: bool,
}

impl ToArgs for RunArgs {
    fn to_args(&self) -> Vec<String> {
        let Self {
            arguments,
            output,
            target,
            print_program_output,
            print_resource_usage,
            experimental_oracles,
        } = self;
        let mut args = arguments.to_args();
        if let Some(output) = output {
            args.push("--output".to_string());
            args.push(output.to_string());
        }
        args.push("--target".to_string());
        args.push(target.to_string());
        if *print_program_output {
            args.push("--print-program-output".to_string());
        }
        if *print_resource_usage {
            args.push("--print-resource-usage".to_string());
        }
        if *experimental_oracles {
            args.push("--experimental-oracles".to_string());
        }
        args
    }
}

/// Arguments to the executable function.
#[derive(Parser, Debug, Clone)]
pub struct ProgramArguments {
    /// Serialized arguments to the executable function.
    #[arg(long, value_delimiter = ',')]
    pub arguments: Vec<Felt252>,

    /// Serialized arguments to the executable function from a file.
    #[arg(long, conflicts_with = "arguments")]
    pub arguments_file: Option<Utf8PathBuf>,
}

impl ToArgs for ProgramArguments {
    fn to_args(&self) -> Vec<String> {
        let Self {
            arguments,
            arguments_file,
        } = self;
        if let Some(arguments_file) = arguments_file {
            return vec!["--arguments-file".to_string(), arguments_file.to_string()];
        }
        if arguments.is_empty() {
            return vec![];
        }
        let arguments = arguments
            .iter()
            .map(|a| a.to_string())
            .collect::<Vec<String>>()
            .join(",");
        vec!["--arguments".to_string(), arguments]
    }
}

/// Output format for the execution
#[derive(ValueEnum, Clone, Debug)]
pub enum OutputFormat {
    /// Output in Cairo PIE (Program Independent Execution) format
    CairoPie,
    /// Output in standard format
    Standard,
    /// No output
    None,
}

#[doc(hidden)]
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
    pub fn is_none(&self) -> bool {
        matches!(self, OutputFormat::None)
    }
}

impl Display for OutputFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OutputFormat::CairoPie => write!(f, "cairo-pie"),
            OutputFormat::Standard => write!(f, "standard"),
            OutputFormat::None => write!(f, "none"),
        }
    }
}

/// Execution target for the program.
#[derive(ValueEnum, Clone, Debug)]
pub enum ExecutionTarget {
    /// Bootloader target.
    Bootloader,
    /// Standalone target.
    Standalone,
}

#[doc(hidden)]
impl ExecutionTarget {
    pub fn is_standalone(&self) -> bool {
        matches!(self, ExecutionTarget::Standalone)
    }
    pub fn is_bootloader(&self) -> bool {
        matches!(self, ExecutionTarget::Bootloader)
    }
}

impl Display for ExecutionTarget {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExecutionTarget::Bootloader => write!(f, "bootloader"),
            ExecutionTarget::Standalone => write!(f, "standalone"),
        }
    }
}

#[doc(hidden)]
pub trait ToArgs {
    /// Convert parsed args to an array of arguments.
    fn to_args(&self) -> Vec<String>;
}
