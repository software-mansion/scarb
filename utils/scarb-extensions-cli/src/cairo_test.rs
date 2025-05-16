use clap::{Parser, ValueEnum};
use scarb_ui::args::{PackagesFilter, VerbositySpec};

/// Execute all unit tests of a local package.
#[derive(Parser, Clone, Debug)]
#[command(author, version)]
pub struct Args {
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

    /// Logging verbosity.
    #[command(flatten)]
    pub verbose: VerbositySpec,
}

#[derive(ValueEnum, Clone, Debug, Default)]
pub enum TestKind {
    Unit,
    Integration,
    #[default]
    All,
}

impl TestKind {
    pub fn matches(&self, kind: &str) -> bool {
        match self {
            TestKind::Unit => kind == "unit",
            TestKind::Integration => kind == "integration",
            TestKind::All => true,
        }
    }
}
