use crate::Verbosity;

/// [`clap`] structured arguments that provide Scarb UI verbosity selection.
#[derive(clap::Args, Debug, Clone, Default)]
#[command(about = None, long_about = None)]
pub struct VerbositySpec {
    #[arg(
    long,
    short = 'v',
    action = clap::ArgAction::Count,
    global = true,
    help = "Increase logging verbosity.",
    )]
    verbose: u8,

    #[arg(
    long,
    short = 'q',
    action = clap::ArgAction::Count,
    global = true,
    help = "Decrease logging verbosity.",
    conflicts_with = "verbose",
    )]
    quiet: u8,

    #[arg(
        long,
        global = true,
        help = "Set UI verbosity level by name.",
        env = "SCARB_UI_VERBOSITY"
    )]
    verbosity: Option<Verbosity>,
}

impl Verbosity {
    fn level_value(level: Self) -> i8 {
        match level {
            Self::Quiet => 0,
            Self::Normal => 2,
            Self::Verbose => 4,
        }
    }
}

impl VerbositySpec {
    /// Whether any verbosity flags (either `--verbose` or `--quiet`)
    /// are present on the command line.
    pub fn is_present(&self) -> bool {
        self.verbose != 0 || self.quiet != 0
    }

    /// Convert the verbosity specification to a [`tracing_core::LevelFilter`].
    pub fn as_trace(&self) -> tracing_core::LevelFilter {
        match self.integer_verbosity() {
            i8::MIN..=-1 => tracing_core::LevelFilter::OFF,
            0 => tracing_core::LevelFilter::ERROR,
            1 => tracing_core::LevelFilter::WARN,
            2 => tracing_core::LevelFilter::INFO,
            3 => tracing_core::LevelFilter::DEBUG,
            4..=i8::MAX => tracing_core::LevelFilter::TRACE,
        }
    }

    fn integer_verbosity(&self) -> i8 {
        let int_level = Verbosity::level_value(Verbosity::default()) - (self.quiet as i8)
            + (self.verbose as i8);
        if self.is_present() {
            int_level
        } else {
            self.verbosity
                .map(Verbosity::level_value)
                .unwrap_or(int_level)
        }
    }
}

impl From<VerbositySpec> for Verbosity {
    fn from(spec: VerbositySpec) -> Self {
        match spec.integer_verbosity() {
            v if v < 2 => Verbosity::Quiet,
            2 => Verbosity::Normal,
            _ => Verbosity::Verbose,
        }
    }
}
