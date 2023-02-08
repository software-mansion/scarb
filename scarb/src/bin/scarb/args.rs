#![deny(missing_docs)]

//! CLI arguments datastructures.

use std::ffi::OsString;

use camino::Utf8PathBuf;
use clap::{Parser, Subcommand};

use scarb::core::PackageName;
use scarb::metadata::MetadataVersion;
use scarb::ui::OutputFormat;

/// Cairo's project manager.
#[derive(Parser, Clone, Debug)]
#[command(author, version, about)]
pub struct Args {
    /// Override path to a directory containing a **Scarb.toml** file.
    #[arg(long, env = "SCARB_MANIFEST_PATH")]
    pub manifest_path: Option<Utf8PathBuf>,

    /// Logging verbosity.
    #[command(flatten)]
    pub verbose: clap_verbosity_flag::Verbosity,

    /// Print machine-readable output in [NDJSON](https://github.com/ndjson/ndjson-spec) format.
    #[arg(long)]
    pub json: bool,

    /// Run without accessing the network.
    #[arg(long, env = "SCARB_OFFLINE")]
    pub offline: bool,

    /// Subcommand and its arguments.
    #[command(subcommand)]
    pub command: Command,
}

impl Args {
    /// Construct [`OutputFormat`] value from these arguments.
    pub fn output_format(&self) -> OutputFormat {
        if self.json {
            OutputFormat::Json
        } else {
            OutputFormat::default()
        }
    }
}

/// Subcommand and its arguments.
#[derive(Subcommand, Clone, Debug)]
pub enum Command {
    // Keep these sorted alphabetically.
    // External should go last.
    /// Add dependencies to a manifest file.
    Add,
    /// Compile current project.
    Build,
    /// Remove generated artifacts.
    Clean,
    /// List installed commands.
    Commands,
    /// Format project files.
    Fmt(FmtArgs),
    /// Create a new Scarb package in existing directory.
    Init(InitArgs),
    /// Print path to current **Scarb.toml** file to standard output.
    ManifestPath,
    /// Output the resolved dependencies of a package, the concrete used versions including
    /// overrides, in machine-readable format.
    Metadata(MetadataArgs),
    /// Create a new Scarb package at <PATH>.
    New(NewArgs),

    /// External command (`scarb-*` executable).
    #[command(external_subcommand)]
    External(Vec<OsString>),
}

/// Arguments accepted by the `init` command.
#[derive(Parser, Clone, Debug)]
pub struct InitArgs {
    /// Set the resulting package name, defaults to the directory name.
    #[arg(long)]
    pub name: Option<PackageName>,
}

/// Arguments accepted by the `metadata` command.
#[derive(Parser, Clone, Debug)]
pub struct MetadataArgs {
    // Format version.
    #[arg(long, value_name = "VERSION")]
    pub format_version: MetadataVersion,
    /// Output information only about the workspace members and don't fetch dependencies.
    #[arg(long)]
    pub no_deps: bool,
}

/// Arguments accepted by the `new` command.
#[derive(Parser, Clone, Debug)]
pub struct NewArgs {
    pub path: Utf8PathBuf,
    #[command(flatten)]
    pub init: InitArgs,
}

/// Arguments accepted by the `fmt` command.
#[derive(Parser, Clone, Debug)]
pub struct FmtArgs {
    /// Only check if files are formatted, do not write the changes to disk.
    #[arg(short, long, default_value_t = false)]
    pub check: bool,
    /// Do not color output.
    #[arg(long, default_value_t = false)]
    pub no_color: bool,
    /// Specify package to format.
    #[arg(short, long, value_name = "PACKAGE")]
    pub package: Option<PackageName>,
}
