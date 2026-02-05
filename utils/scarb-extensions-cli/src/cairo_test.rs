#![deny(missing_docs)]

//! Extension CLI arguments datastructures.

use clap::{Parser, ValueEnum};
use scarb_ui::OutputFormat;
use scarb_ui::args::{PackagesFilter, VerbositySpec};

/// CLI command name.
pub const COMMAND_NAME: &str = "cairo-test";

/// Execute all unit tests of a local package.
#[derive(Parser, Clone, Debug)]
#[command(name = COMMAND_NAME, author, version)]
pub struct Args {
    /// Specify package(s) to operate on.
    #[command(flatten)]
    pub packages_filter: PackagesFilter,

    /// Run only tests whose name contain FILTER.
    #[arg(short, long, default_value = "")]
    pub filter: String,

    /// Run ignored and not ignored tests.
    #[arg(long, default_value_t = false)]
    pub include_ignored: bool,

    /// Run only ignored tests.
    #[arg(long, default_value_t = false)]
    pub ignored: bool,

    /// Choose test kind to run.
    #[arg(short, long)]
    pub test_kind: Option<TestKind>,

    /// Whether to print resource usage after each test.
    #[arg(long, default_value_t = false)]
    pub print_resource_usage: bool,

    /// Print machine-readable output in NDJSON format.
    #[arg(long, env = "SCARB_OUTPUT_JSON")]
    pub json: bool,

    /// Logging verbosity.
    #[command(flatten)]
    pub verbose: VerbositySpec,
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

/// Test kind to run.
#[derive(ValueEnum, Clone, Debug, Default)]
pub enum TestKind {
    /// Run only unit tests.
    Unit,
    /// Run only integration tests.
    Integration,
    /// Run all tests.
    #[default]
    All,
}

#[doc(hidden)]
impl TestKind {
    pub fn matches(&self, kind: &str) -> bool {
        match self {
            TestKind::Unit => kind == "unit",
            TestKind::Integration => kind == "integration",
            TestKind::All => true,
        }
    }
}
