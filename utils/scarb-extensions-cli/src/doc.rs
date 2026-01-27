#![deny(missing_docs)]

//! Extension CLI arguments datastructures.

use clap::Parser;
use scarb_ui::args::{FeaturesSpec, PackagesFilter, VerbositySpec};

/// CLI command name.
pub const COMMAND_NAME: &str = "doc";

/// Format of generated documentation files.
#[derive(Default, Debug, Clone, clap::ValueEnum)]
pub enum OutputFormat {
    /// Generates documentation in Markdown format.
    /// Generated files are fully compatible with mdBook. For more information visit <https://rust-lang.github.io/mdBook>.
    #[default]
    Markdown,
    /// Saves information collected from packages in JSON format instead of generating
    /// documentation.
    /// This may be useful if you want to generate documentation files by yourself.
    /// The precise output structure is not guaranteed to be stable.
    Json,
    /// Generates documentation alike Markdown format with `.mdx` extension instead.
    #[value(hide = true)] // do not advertise in --help but accept from CLI
    Mdx,
}

/// Generate documentation based on code comments
#[derive(Parser, Debug)]
#[command(name = COMMAND_NAME, version, about, long_about = None)]
pub struct Args {
    /// Specify package(s) to operate on.
    #[command(flatten)]
    pub packages_filter: PackagesFilter,

    /// Specifies a format of generated files.
    #[arg(long, value_enum, default_value_t)]
    pub output_format: OutputFormat,

    /// Generates documentation also for private items.
    #[arg(long, default_value_t = false)]
    pub document_private_items: bool,

    /// Build generated documentation.
    #[arg(long, default_value_t = false)]
    pub build: bool,

    /// Open the generated documentation in a browser.
    ///
    /// Implies `--build`.
    #[arg(long, default_value_t = false)]
    pub open: bool,

    /// Specifies features to enable.
    #[command(flatten)]
    pub features: FeaturesSpec,

    /// Print machine-readable output in NDJSON format.
    #[arg(long, env = "SCARB_OUTPUT_JSON")]
    pub json: bool,

    /// Logging verbosity.
    #[command(flatten)]
    pub verbose: VerbositySpec,

    /// Base URL of a remote repository. Used for generating links to source code.
    #[arg(long, env = "SCARB_DOC_REMOTE_BASE_URL")]
    pub remote_base_url: Option<String>,

    /// Enables/disables linking documentation items to the source code repository.
    #[arg(long, default_value_t = false)]
    pub disable_remote_linking: bool,
}

impl Args {
    /// Construct [`scarb_ui::OutputFormat`] value from these arguments.
    pub fn ui_output_format(&self) -> scarb_ui::OutputFormat {
        if self.json {
            scarb_ui::OutputFormat::Json
        } else {
            scarb_ui::OutputFormat::default()
        }
    }
}
