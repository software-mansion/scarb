#![deny(missing_docs)]

//! CLI arguments datastructures.

use std::ffi::OsString;

use camino::Utf8PathBuf;
use clap::{Parser, Subcommand};
use tracing::level_filters::LevelFilter;
use tracing_log::AsTrace;

use scarb::core::PackageName;
use scarb::manifest_editor::DepId;
use scarb::metadata::MetadataVersion;
use scarb::ui;
use scarb::ui::OutputFormat;
use scarb::version;

/// The Cairo package manager.
#[derive(Parser, Clone, Debug)]
#[command(author, version = version::get().short(), long_version = version::get().long())]
pub struct ScarbArgs {
    /// Override path to a directory containing a Scarb.toml file.
    #[arg(long, env = "SCARB_MANIFEST_PATH", global = true)]
    pub manifest_path: Option<Utf8PathBuf>,

    /// Logging verbosity.
    #[command(flatten)]
    pub verbose: clap_verbosity_flag::Verbosity,

    /// Print machine-readable output in NDJSON format.
    #[arg(long, global = true)]
    pub json: bool,

    /// Run without accessing the network.
    #[arg(long, env = "SCARB_OFFLINE", global = true)]
    pub offline: bool,

    /// Directory for all generated artifacts.
    #[arg(
        long,
        env = "SCARB_TARGET_DIR",
        value_name = "DIRECTORY",
        global = true
    )]
    pub target_dir: Option<Utf8PathBuf>,

    /// Subcommand and its arguments.
    #[command(subcommand)]
    pub command: Command,
}

impl ScarbArgs {
    /// Construct [`OutputFormat`] value from these arguments.
    pub fn output_format(&self) -> OutputFormat {
        if self.json {
            OutputFormat::Json
        } else {
            OutputFormat::default()
        }
    }

    /// Get [`ui::Verbosity`] out of this arguments.
    pub fn ui_verbosity(&self) -> ui::Verbosity {
        let filter = self.verbose.log_level_filter().as_trace();
        if filter >= LevelFilter::WARN {
            ui::Verbosity::Verbose
        } else if filter > LevelFilter::OFF {
            ui::Verbosity::Normal
        } else {
            ui::Verbosity::Quiet
        }
    }
}

/// Subcommand and its arguments.
#[derive(Subcommand, Clone, Debug)]
pub enum Command {
    // Keep these sorted alphabetically.
    // External should go last.
    /// Add dependencies to a Scarb.toml manifest file.
    Add(AddArgs),
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
    /// Print path to current Scarb.toml file to standard output.
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
    #[arg(short, long)]
    pub package: Option<PackageName>,
}

/// Arguments accepted by the `add` command.
#[derive(Parser, Clone, Debug)]
pub struct AddArgs {
    /// Reference to a package to add as a dependency
    ///
    /// You can reference a package by:
    /// - `<name>`, like `scarb add quaireaux` (the latest version will be used)
    /// - `<name>@<version-req>`, like `scarb add quaireaux@1` or `scarb add quaireaux@=0.1.0`
    #[arg(value_name = "DEP_ID", verbatim_doc_comment)]
    pub packages: Vec<DepId>,

    /// Do not actually write the manifest.
    #[arg(long)]
    pub dry_run: bool,

    /// Specify package to modify.
    #[arg(short, long)]
    pub package: Option<PackageName>,

    /// _Source_ section.
    #[command(flatten, next_help_heading = "Source")]
    pub source: AddSourceArgs,
}

/// _Source_ section of [`AddArgs`].
#[derive(Parser, Clone, Debug)]
pub struct AddSourceArgs {
    /// Filesystem path to local package to add.
    #[arg(long, conflicts_with_all = ["git", "GitRefGroup"])]
    pub path: Option<Utf8PathBuf>,

    /// Git repository location
    ///
    /// Without any other information, Scarb will use the latest commit on the default branch.
    #[arg(long, value_name = "URI")]
    pub git: Option<String>,

    /// Git reference args for `--git`.
    #[command(flatten)]
    pub git_ref: GitRefGroup,
}

/// Git reference specification arguments.
#[derive(Parser, Clone, Debug)]
pub struct GitRefGroup {
    /// Git branch to download the package from.
    #[arg(long, requires = "git", conflicts_with_all = ["tag", "rev"])]
    pub branch: Option<String>,

    /// Git tag to download the package from.
    #[arg(long, requires = "git", conflicts_with_all = ["branch", "rev"])]
    pub tag: Option<String>,

    /// Git reference to download the package from
    ///
    /// This is the catch-all, handling hashes to named references in remote repositories.
    #[arg(long, requires = "git")]
    pub rev: Option<String>,
}

#[cfg(test)]
mod tests {
    use clap::CommandFactory;

    use super::ScarbArgs;

    #[test]
    fn verify() {
        ScarbArgs::command().debug_assert();
    }
}
