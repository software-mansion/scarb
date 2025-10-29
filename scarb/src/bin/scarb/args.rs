#![deny(missing_docs)]

//! CLI arguments datastructures.

use std::collections::BTreeMap;
use std::ffi::OsString;

use camino::Utf8PathBuf;
use clap::{CommandFactory, Parser, Subcommand, ValueEnum};
use smol_str::SmolStr;
use url::Url;

use clap_complete::Shell;
use scarb::core::PackageName;
use scarb::manifest_editor::DepId;
use scarb::manifest_editor::SectionArgs;
use scarb::version;
use scarb_ui::OutputFormat;
use scarb_ui::args::{FeaturesSpec, PackagesFilter, ProfileSpec, VerbositySpec};

/// The Cairo package manager.
#[derive(Parser, Clone, Debug)]
#[command(
    author,
    version = version::get().short(),
    long_version = version::get().long(),
    help_template = "\
{name} {version}
{author-with-newline}{about-with-newline}
Use -h for short descriptions and --help for more details.

{before-help}{usage-heading} {usage}

{all-args}{after-help}
",
    long_about = "Scarb is the Cairo package manager. It downloads your package's dependencies, compiles your \
    projects, and works as an entry point for other tooling to work with your code.",
    after_help = "Read the docs: https://docs.swmansion.com/scarb/",
    after_long_help = "\
Read the docs:
- Scarb Book: https://docs.swmansion.com/scarb/docs.html
- Cairo Book: https://book.cairo-lang.org/
- Starknet Documentation: https://docs.starknet.io/

Join the community:
- Follow us on @swmansionxyz: https://twitter.com/swmansionxyz
- Chat on Telegram: https://t.me/+1pMLtrNj5NthZWJk
- Contact us privately on Telegram: @scarb_chat
- Socialize on Starknet's Discord: https://discord.gg/KZWaFtPZJf

Report bugs: https://github.com/software-mansion/scarb/issues/new/choose\
",
)]
pub struct ScarbArgs {
    /// Path to Scarb.toml.
    #[arg(long, env = "SCARB_MANIFEST_PATH", hide_short_help = true)]
    pub manifest_path: Option<Utf8PathBuf>,

    /// Logging verbosity.
    #[command(flatten)]
    pub verbose: VerbositySpec,

    /// Print machine-readable output in NDJSON format.
    #[arg(long, env = "SCARB_OUTPUT_JSON")]
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

    /// Do not load any procedural macros.
    #[arg(long, env = "SCARB_NO_PROC_MACROS")]
    pub no_proc_macros: bool,

    /// Do not load any prebuilt procedural macros.
    #[arg(long, env = "SCARB_NO_PREBUILT_PROC_MACROS")]
    pub no_prebuilt_proc_macros: bool,

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
    /// Fetch and list available versions of a package from the registry.
    List(ListCommandArgs),
    /// Compile current project.
    Build(BuildArgs),
    /// Expand macros.
    Expand(ExpandArgs),
    /// Manipulate packages cache.
    #[clap(subcommand)]
    Cache(CacheSubcommand),
    /// Analyze the current package and report errors, but don't build Sierra files.
    Check(BuildArgs),
    /// Remove generated artifacts.
    Clean,
    /// Generate shell completions for Scarb.
    Completions(CompletionsArgs),
    /// List installed commands.
    Commands,
    /// Fetch dependencies of packages from the network.
    Fetch,
    /// Format project files.
    Fmt(FmtArgs),
    /// Create a new Scarb package in the existing directory.
    Init(InitArgs),
    /// Print a path to the current Scarb.toml file to standard output.
    ManifestPath,
    /// Output the resolved dependencies of a package, the concrete versions used, including
    /// overrides, in machine-readable format.
    Metadata(MetadataArgs),
    /// Create a new Scarb package at PATH.
    New(NewArgs),
    /// Display a tree visualisation of a dependency graph.
    #[command(after_help = "\
        WARNING: The JSON output of this command is unstable across Scarb releases.
    ")]
    Tree(TreeCommandArgs),
    /// Assemble the local package into a distributable tarball.
    #[command(after_help = "\
        This command will create distributable, compressed `.tar.zst` archives containing source \
        codes of selected packages. Resulting files will be placed in `target/package` directory.
    ")]
    Package(PackageArgs),
    /// Start the proc macro server.
    ProcMacroServer,
    /// Upload a package to the registry.
    #[command(after_help = "\
        This command will create distributable, compressed `.tar.zst` archive containing source \
        code of the package in `target/package` directory (using `scarb package`) and upload it \
        to a registry.
    ")]
    Publish(PublishArgs),
    /// Checks a package to catch common mistakes and improve your Cairo code.
    Lint(LintArgs),
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
    /// Update dependencies.
    Update,
    /// External command (`scarb-*` executable).
    #[command(external_subcommand)]
    External(Vec<OsString>),
}

#[derive(ValueEnum, Clone, Debug)]
pub enum EmitTarget {
    Stdout,
}

/// Arguments accepted by the `build` command.
#[derive(Parser, Clone, Debug)]
pub struct BuildArgs {
    /// Specify package(s) to operate on.
    #[command(flatten)]
    pub packages_filter: PackagesFilter,

    /// Build tests.
    #[arg(short, long, default_value_t = false)]
    pub test: bool,

    /// Comma separated list of target names to compile.
    #[arg(long, value_delimiter = ',', env = "SCARB_TARGET_NAMES")]
    pub target_names: Vec<String>,

    /// Comma separated list of target kinds to compile.
    #[arg(
        long,
        value_delimiter = ',',
        env = "SCARB_TARGET_KINDS",
        conflicts_with_all = ["target_names", "test"]
    )]
    pub target_kinds: Vec<String>,

    /// Specify features to enable.
    #[command(flatten)]
    pub features: FeaturesSpec,

    /// Do not error on `cairo-version` mismatch.
    #[arg(long, env = "SCARB_IGNORE_CAIRO_VERSION")]
    pub ignore_cairo_version: bool,
}

/// Arguments accepted by the `expand` command.
#[derive(Parser, Clone, Debug)]
pub struct ExpandArgs {
    /// Specify package(s) to operate on.
    #[command(flatten)]
    pub packages_filter: PackagesFilter,

    /// Specify features to enable.
    #[command(flatten)]
    pub features: FeaturesSpec,

    /// Do not error on `cairo-version` mismatch.
    #[arg(long, env = "SCARB_IGNORE_CAIRO_VERSION")]
    pub ignore_cairo_version: bool,

    /// Specify the target to expand by target kind.
    #[arg(long)]
    pub target_kind: Option<String>,

    /// Specify the target to expand by target name.
    #[arg(long)]
    pub target_name: Option<String>,

    /// Do not attempt formatting.
    #[arg(long, default_value_t = false)]
    pub ugly: bool,

    /// Emit the expanded file to stdout
    #[arg(short, long)]
    pub emit: Option<EmitTarget>,
}

/// Arguments accepted by the `run` command.
#[derive(Parser, Clone, Debug)]
#[clap(trailing_var_arg = true)]
pub struct ScriptsRunnerArgs {
    /// The name of the script from the manifest file to execute.
    pub script: Option<SmolStr>,

    /// Specify package(s) to operate on.
    #[command(flatten)]
    pub packages_filter: PackagesFilter,

    /// Run the script in workspace root only.
    #[arg(long, default_value_t = false)]
    pub workspace_root: bool,

    /// Arguments to pass to the executed script.
    #[clap(allow_hyphen_values = true)]
    pub args: Vec<OsString>,
}

/// Specifies the test runner to use for running tests.
#[derive(ValueEnum, Clone, Debug)]
pub enum TestRunner {
    /// Uses the `Starknet Foundry` test runner.
    StarknetFoundry,
    /// No test runner.
    None,
}

/// Arguments accepted by the `init` command.
#[derive(Parser, Clone, Debug)]
pub struct InitArgs {
    /// Set the resulting package name, defaults to the directory name.
    #[arg(long)]
    pub name: Option<PackageName>,

    /// Do not initialize a new Git repository.
    #[arg(long, env = "SCARB_INIT_NO_VCS")]
    pub no_vcs: bool,

    /// Test runner to use. Starts interactive session if not specified.
    #[arg(long, env = "SCARB_INIT_TEST_RUNNER")]
    pub test_runner: Option<TestRunner>,
}

/// Arguments accepted by the `metadata` command.
#[derive(Parser, Clone, Debug)]
pub struct MetadataArgs {
    /// Format version.
    #[arg(long, value_name = "VERSION")]
    pub format_version: u64,
    /// Output information only about the workspace members and don't fetch dependencies.
    #[arg(long)]
    pub no_deps: bool,

    /// Specify features to enable.
    #[command(flatten)]
    pub features: FeaturesSpec,

    /// Do not error on `cairo-version` mismatch.
    #[arg(long, env = "SCARB_IGNORE_CAIRO_VERSION")]
    pub ignore_cairo_version: bool,
}

/// Arguments accepted by the `new` command.
#[derive(Parser, Clone, Debug)]
pub struct NewArgs {
    /// Path to the new package directory.
    pub path: Utf8PathBuf,
    /// Initialization options.
    #[command(flatten)]
    pub init: InitArgs,
}

/// Arguments accepted by the `fmt` command.
#[derive(Parser, Clone, Debug)]
pub struct FmtArgs {
    /// Only check if files are formatted, do not write the changes to disk.
    #[arg(short, long, default_value_t = false, conflicts_with = "emit")]
    pub check: bool,
    /// Emit the formatted file to stdout
    #[arg(short, long)]
    pub emit: Option<EmitTarget>,
    /// Do not color output.
    #[arg(long, default_value_t = false, env = "SCARB_FMT_NO_COLOR")]
    pub no_color: bool,
    /// Specify package(s) to format.
    #[command(flatten)]
    pub packages_filter: PackagesFilter,
    /// Path to a file or directory to format. If provided, only this file or directory will be formatted.
    #[clap(value_name = "SCARB_ACTION_PATH")]
    pub path: Option<Utf8PathBuf>,
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

    /// Specify package(s) to operate on.
    #[command(flatten)]
    pub packages_filter: PackagesFilter,

    /// _Source_ section.
    #[command(flatten, next_help_heading = "Source")]
    pub source: AddSourceArgs,

    /// _Section_ section.
    #[command(flatten, next_help_heading = "Section")]
    pub section: AddSectionArgs,
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

/// _Section_ section of [`AddArgs`].
#[derive(Parser, Clone, Debug)]
pub struct AddSectionArgs {
    /// Add as development dependency.
    ///
    /// Dev-dependencies are only used when compiling tests.
    ///
    /// These dependencies are not propagated to other packages which depend on this package.
    #[arg(long)]
    pub dev: bool,
}

impl SectionArgs for AddSectionArgs {
    fn dev(&self) -> bool {
        self.dev
    }
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

    /// Specify package(s) to operate on.
    #[command(flatten)]
    pub packages_filter: PackagesFilter,

    /// _Section_ section.
    #[command(flatten, next_help_heading = "Section")]
    pub section: RemoveSectionArgs,
}

/// _Section_ section of [`RemoveArgs`].
#[derive(Parser, Clone, Debug)]
pub struct RemoveSectionArgs {
    /// Remove as development dependency.
    #[arg(long)]
    pub dev: bool,
}

impl SectionArgs for RemoveSectionArgs {
    fn dev(&self) -> bool {
        self.dev
    }
}

/// Arguments accepted by the `test` command.
#[derive(Parser, Clone, Debug)]
pub struct TestArgs {
    /// Specify package(s) to operate on.
    #[command(flatten)]
    pub packages_filter: PackagesFilter,

    /// Arguments for the test program.
    #[clap(allow_hyphen_values = true)]
    pub args: Vec<OsString>,

    /// Specify features to enable.
    #[command(flatten)]
    pub features: FeaturesSpec,
}

/// Arguments accepted by both the `package` and the `publish` command.
#[derive(Parser, Clone, Debug)]
pub struct PackageSharedArgs {
    /// Allow working directories with uncommitted VCS changes to be packaged.
    #[arg(long, env = "SCARB_PACKAGE_ALLOW_DIRTY")]
    pub allow_dirty: bool,

    /// Do not verify the contents by building them.
    #[arg(long, env = "SCARB_PACKAGE_NO_VERIFY")]
    pub no_verify: bool,
}

/// Arguments accepted by the `package` command.
#[derive(Parser, Clone, Debug)]
pub struct PackageArgs {
    /// Print files included in a package without making one.
    #[arg(short, long)]
    pub list: bool,

    /// Ignore warnings about a lack of human-usable metadata
    #[arg(long)]
    pub no_metadata: bool,

    /// Shared packaging arguments.
    #[clap(flatten)]
    pub shared_args: PackageSharedArgs,

    /// Specify package(s) to operate on.
    #[command(flatten)]
    pub packages_filter: PackagesFilter,

    /// Specify features to enable.
    #[command(flatten)]
    pub features: FeaturesSpec,

    /// Do not error on `cairo-version` mismatch.
    #[arg(long, env = "SCARB_IGNORE_CAIRO_VERSION")]
    pub ignore_cairo_version: bool,
}

/// Arguments accepted by the `publish` command.
#[derive(Parser, Clone, Debug)]
pub struct PublishArgs {
    /// Registry index URL to upload the package to.
    #[arg(long, value_name = "URL")]
    pub index: Option<Url>,

    /// Shared packaging arguments.
    #[clap(flatten)]
    pub shared_args: PackageSharedArgs,

    /// Specify package(s) to operate on.
    #[command(flatten)]
    pub packages_filter: PackagesFilter,

    /// Specify features to enable.
    #[command(flatten)]
    pub features: FeaturesSpec,

    /// Do not error on `cairo-version` mismatch.
    #[arg(long, env = "SCARB_IGNORE_CAIRO_VERSION")]
    pub ignore_cairo_version: bool,
}

/// Arguments accepted by the `lint` command.
#[derive(Parser, Clone, Debug)]
pub struct LintArgs {
    /// Specify package(s) to operate on.
    #[command(flatten)]
    pub packages_filter: PackagesFilter,

    /// Comma separated list of target names to compile.
    #[arg(long, value_delimiter = ',', env = "SCARB_TARGET_NAMES")]
    pub target_names: Vec<String>,

    /// Should lint the tests.
    #[arg(short, long, default_value_t = false)]
    pub test: bool,

    /// Should fix the lint when it can.
    #[arg(short, long, default_value_t = false)]
    pub fix: bool,

    /// Do not error on `cairo-version` mismatch.
    #[arg(long, env = "SCARB_IGNORE_CAIRO_VERSION")]
    pub ignore_cairo_version: bool,

    /// Specify features to enable.
    #[command(flatten)]
    pub features: FeaturesSpec,

    /// Should fail on any warning.
    #[arg(short, long, default_value_t = false, env = "SCARB_LINT_DENY_WARNINGS")]
    pub deny_warnings: bool,

    /// Path to a file or directory to lint. If provided, only this file or directory will be linted.
    #[clap(value_name = "SCARB_ACTION_PATH")]
    pub path: Option<Utf8PathBuf>,
}

/// Arguments accepted by the `completions` command.
#[derive(Parser, Clone, Debug)]
#[clap(version, about = "Generate shell completions")]
pub struct CompletionsArgs {
    /// Target shell for completion generation
    #[arg(value_enum)]
    pub shell: Option<Shell>,
}

/// Arguments accepted by the `tree` command.
#[derive(Parser, Clone, Debug)]
pub struct TreeCommandArgs {
    /// Specify the package(s) to operate on.
    #[command(flatten)]
    pub packages_filter: PackagesFilter,

    /// Prune the given package(s) from the display of the dependency tree.
    #[arg(long)]
    pub prune: Vec<PackageName>,

    /// Maximum display depth of the dependency tree.
    #[arg(long)]
    pub depth: Option<usize>,

    /// Do not de-duplicate repeated dependencies.
    #[arg(long)]
    pub no_dedupe: bool,

    /// Include the `core` package in the dependency tree.
    #[arg(long)]
    pub core: bool,
}

/// Arguments accepted by the `list` command.
#[derive(Parser, Clone, Debug)]
pub struct ListCommandArgs {
    /// Package name.
    #[arg(value_name = "PACKAGE_NAME")]
    pub package_name: PackageName,

    /// Registry index URL to query package versions from.
    #[arg(long, value_name = "URL")]
    pub index: Option<Url>,

    /// Show all available versions.
    #[arg(long, default_value_t = false)]
    pub all: bool,

    /// Limit the number of results.
    #[arg(long, default_value_t = 10, conflicts_with = "all")]
    pub limit: usize,
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

#[cfg(test)]
mod tests {
    use clap::CommandFactory;

    use super::ScarbArgs;

    #[test]
    fn verify() {
        ScarbArgs::command().debug_assert();
    }
}
