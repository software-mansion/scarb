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
            Self::Quiet => -1,
            Self::Normal => 0,
            Self::Verbose => 1,
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
    pub fn as_trace(&self) -> String {
        let level = match self.integer_verbosity() {
            i8::MIN..=-1 => tracing_core::LevelFilter::OFF,
            0 => tracing_core::LevelFilter::ERROR,
            1 => tracing_core::LevelFilter::WARN,
            2 => tracing_core::LevelFilter::INFO,
            3 => tracing_core::LevelFilter::DEBUG,
            4..=i8::MAX => tracing_core::LevelFilter::TRACE,
        };
        format!("scarb={level}")
    }

    fn integer_verbosity(&self) -> i8 {
        let int_level = (self.verbose as i8) - (self.quiet as i8);
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
            v if v < 0 => Verbosity::Quiet,
            0 => Verbosity::Normal,
            _ => Verbosity::Verbose,
        }
    }
}

#[cfg(test)]
mod tests {
    use test_case::test_case;

    use crate::args::VerbositySpec;
    use crate::Verbosity;

    #[test_case(Verbosity::Quiet)]
    #[test_case(Verbosity::Normal)]
    #[test_case(Verbosity::Verbose)]
    fn verbosity_serialization_identity(level: Verbosity) {
        assert_eq!(
            Verbosity::from(VerbositySpec {
                verbose: 0,
                quiet: 0,
                verbosity: Some(level),
            }),
            level
        );
    }

    #[test_case(2, 0, Verbosity::Quiet, tracing_core::LevelFilter::OFF)]
    #[test_case(1, 0, Verbosity::Quiet, tracing_core::LevelFilter::OFF)]
    #[test_case(0, 0, Verbosity::Normal, tracing_core::LevelFilter::ERROR)]
    #[test_case(0, 1, Verbosity::Verbose, tracing_core::LevelFilter::WARN)]
    #[test_case(0, 2, Verbosity::Verbose, tracing_core::LevelFilter::INFO)]
    #[test_case(0, 3, Verbosity::Verbose, tracing_core::LevelFilter::DEBUG)]
    #[test_case(0, 4, Verbosity::Verbose, tracing_core::LevelFilter::TRACE)]
    #[test_case(0, 5, Verbosity::Verbose, tracing_core::LevelFilter::TRACE)]
    fn verbosity_levels(
        quiet: u8,
        verbose: u8,
        level: Verbosity,
        trace: tracing_core::LevelFilter,
    ) {
        let spec = VerbositySpec {
            verbose,
            quiet,
            verbosity: None,
        };
        assert_eq!(spec.as_trace(), format!("scarb={trace}"));
        assert_eq!(Verbosity::from(spec), level);
    }
}
