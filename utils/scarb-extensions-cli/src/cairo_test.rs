#![deny(missing_docs)]

//! Extension CLI arguments datastructures.

use clap::{Parser, ValueEnum};
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

    /// Enable experimental oracles support.
    #[arg(long, default_value_t = false, env = "SCARB_EXPERIMENTAL_ORACLES")]
    pub experimental_oracles: bool,

    /// Logging verbosity.
    #[command(flatten)]
    pub verbose: VerbositySpec,
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
