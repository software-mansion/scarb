#![deny(missing_docs)]

//! CLI arguments datastructures.

use std::collections::BTreeMap;
use std::ffi::OsString;

use anyhow::Result;
use camino::Utf8PathBuf;
use clap::{CommandFactory, Parser, Subcommand};
use smol_str::SmolStr;
use tracing::level_filters::LevelFilter;
use tracing_log::AsTrace;

use scarb::compiler::Profile;
use scarb::core::PackageName;
use scarb::manifest_editor::DepId;
use scarb::version;
use scarb_ui::args::PackagesFilter;
use scarb_ui::OutputFormat;

/// The Cairo package manager.
#[derive(Parser, Clone, Debug)]
#[command(author, version = version::get().short(), long_version = version::get().long())]
pub struct ScarbArgs {
    /// Path to Scarb.toml.
    #[arg(long, env = "SCARB_MANIFEST_PATH", hide_short_help = true)]
    pub manifest_path: Option<Utf8PathBuf>,

    /// Logging verbosity.
    #[command(flatten)]
    pub verbose: clap_verbosity_flag::Verbosity,

    /// Print machine-readable output in NDJSON format.
    #[arg(long)]
    pub json: bool,

    /// Run without accessing the network.
    #[arg(long, env = "SCARB_OFFLINE", hide_short_help = true)]
    pub offline: bool,

    /// Directory for all cache data stored by Scarb.
    #[arg(
        long,
        env = "SCARB_CACHE",
        value_name = "DIRECTORY",
        hide_short_help = true
    )]
    pub global_cache_dir: Option<Utf8PathBuf>,

    /// Directory for global Scarb configuration files.
    #[arg(
        long,
        env = "SCARB_CONFIG",
        value_name = "DIRECTORY",
        hide_short_help = true
    )]
    pub global_config_dir: Option<Utf8PathBuf>,

    /// Directory for all generated artifacts.
    #[arg(
        long,
        env = "SCARB_TARGET_DIR",
        value_name = "DIRECTORY",
        hide_short_help = true
    )]
    pub target_dir: Option<Utf8PathBuf>,

    /// Specify the profile to use.
    #[command(flatten)]
    pub profile_spec: ProfileSpec,

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

    /// Get [`ui::Verbosity`] out of these arguments.
    pub fn ui_verbosity(&self) -> scarb_ui::Verbosity {
        let filter = self.verbose.log_level_filter().as_trace();
        if filter >= LevelFilter::WARN {
            scarb_ui::Verbosity::Verbose
        } else if filter > LevelFilter::OFF {
            scarb_ui::Verbosity::Normal
        } else {
            scarb_ui::Verbosity::Quiet
        }
    }

    pub fn get_builtin_subcommands() -> BTreeMap<String, Option<String>> {
        Self::command()
            .get_subcommands()
            .map(|sub| {
                let name = sub.get_name().to_string();
                let about = sub.get_about().map(|s| s.to_string());
                (name, about)
            })
            .collect()
    }
}

/// Cache subcommand and its arguments.
#[derive(Subcommand, Clone, Debug)]
pub enum CacheSubcommand {
    /// Remove all cached dependencies.
    Clean,
    /// Print the path of the cache directory.
    Path,
}

/// Subcommand and its arguments.
#[derive(Subcommand, Clone, Debug)]
pub enum Command {
    // Keep these sorted alphabetically.
    // External should go last.
    /// Add dependencies to a Scarb.toml manifest file.
    Add(AddArgs),
    /// Remove dependencies from a manifest file.
    #[command(alias = "rm")]
    Remove(RemoveArgs),
    /// Compile current project.
    Build(BuildArgs),
    /// Manipulate packages cache.
    #[clap(subcommand)]
    Cache(CacheSubcommand),
    /// Remove generated artifacts.
    Clean,
    /// List installed commands.
    Commands,
    /// Fetch dependencies of packages from the network.
    Fetch,
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
    /// Run arbitrary package scripts.
    Run(ScriptsRunnerArgs),
    /// Execute all unit and integration tests of a local package.
    #[command(after_help = "\
        By default, this command delegates to `scarb cairo-test`. This behaviour can be changed by \
        defining a script named `test` in workspace Scarb.toml file.\
        \n\
        Run `scarb test -- --help` for test program options.
    ")]
    Test(TestArgs),

    /// External command (`scarb-*` executable).
    #[command(external_subcommand)]
    External(Vec<OsString>),
}

/// Arguments accepted by the `build` command.
#[derive(Parser, Clone, Debug)]
pub struct BuildArgs {
    #[command(flatten)]
    pub packages_filter: PackagesFilter,
}

/// Arguments accepted by the `run` command.
#[derive(Parser, Clone, Debug)]
#[clap(trailing_var_arg = true)]
pub struct ScriptsRunnerArgs {
    /// The name of the script from manifest file to execute.
    pub script: Option<SmolStr>,

    #[command(flatten)]
    pub packages_filter: PackagesFilter,

    /// Arguments to pass to executed script.
    #[clap(allow_hyphen_values = true)]
    pub args: Vec<OsString>,
}

/// Arguments accepted by the `init` command.
#[derive(Parser, Clone, Debug)]
pub struct InitArgs {
    /// Set the resulting package name, defaults to the directory name.
    #[arg(long)]
    pub name: Option<PackageName>,

    /// Do not initialize a new Git repository.
    #[arg(long)]
    pub no_vcs: bool,
}

/// Arguments accepted by the `metadata` command.
#[derive(Parser, Clone, Debug)]
pub struct MetadataArgs {
    // Format version.
    #[arg(long, value_name = "VERSION")]
    pub format_version: u64,
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
    /// Specify package(s) to format.
    #[command(flatten)]
    pub packages_filter: PackagesFilter,
}

/// Arguments accepted by the `add` command.
#[derive(Parser, Clone, Debug)]
pub struct AddArgs {
    /// Reference to a package to add as a dependency
    ///
    /// You can reference a package by:
    /// - `<name>`, like `scarb add alexandria_math` (the latest version will be used)
    /// - `<name>@<version-req>`, like `scarb add alexandria_math@1` or `scarb add alexandria_math@=0.1.0`
    #[arg(value_name = "DEP_ID", verbatim_doc_comment)]
    pub packages: Vec<DepId>,

    /// Do not actually write the manifest.
    #[arg(long)]
    pub dry_run: bool,

    #[command(flatten)]
    pub packages_filter: PackagesFilter,

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

/// Arguments accepted by the `remove` command.
#[derive(Parser, Clone, Debug)]
pub struct RemoveArgs {
    /// Dependencies to be removed.
    #[arg(value_name = "DEP_ID", required = true)]
    pub packages: Vec<PackageName>,

    /// Do not actually write the manifest.
    #[arg(long)]
    pub dry_run: bool,

    #[command(flatten)]
    pub packages_filter: PackagesFilter,
}

/// Arguments accepted by the `test` command.
#[derive(Parser, Clone, Debug)]
pub struct TestArgs {
    #[command(flatten)]
    pub packages_filter: PackagesFilter,

    /// Arguments for the test program.
    #[clap(allow_hyphen_values = true)]
    pub args: Vec<OsString>,
}

/// Git reference specification arguments.
#[derive(Parser, Clone, Debug)]
#[group(requires = "git", multiple = false)]
pub struct GitRefGroup {
    /// Git branch to download the package from.
    #[arg(long)]
    pub branch: Option<String>,

    /// Git tag to download the package from.
    #[arg(long)]
    pub tag: Option<String>,

    /// Git reference to download the package from
    ///
    /// This is the catch-all, handling hashes to named references in remote repositories.
    #[arg(long)]
    pub rev: Option<String>,
}

/// Profile specifier.
#[derive(Parser, Clone, Debug)]
#[group(multiple = false)]
pub struct ProfileSpec {
    /// Specify profile to use by name.
    #[arg(short = 'P', long)]
    pub profile: Option<SmolStr>,
    /// Use release profile.
    #[arg(long, hide_short_help = true)]
    pub release: bool,
    /// Use dev profile.
    #[arg(long, hide_short_help = true)]
    pub dev: bool,
}

impl ProfileSpec {
    pub fn determine(&self) -> Result<Profile> {
        Ok(match &self {
            Self { release: true, .. } => Profile::RELEASE,
            Self { dev: true, .. } => Profile::DEV,
            Self {
                profile: Some(profile),
                ..
            } => Profile::new(profile.clone())?,
            _ => Profile::default(),
        })
    }
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
