use std::env;
use std::fmt::Display;
use anyhow::{Result, bail};
use clap::ValueEnum;

/// The requested verbosity of output.
///
/// # Ordering
/// [`Verbosity::Quiet`] < [`Verbosity::NoWarnings`] < [`Verbosity::Normal`] < [`Verbosity::Verbose`]
#[derive(ValueEnum, Copy, Clone, Debug, Default, Eq, PartialEq, Ord, PartialOrd)]
pub enum Verbosity {
    /// Avoid printing anything to standard output.
    ///
    /// String representation: `quiet`.
    Quiet,
    /// Avoid printing warnings to standard output.
    ///
    /// String representation: `no-warnings`.
    NoWarnings,
    /// Default verbosity level.
    ///
    /// String representation: `normal`.
    #[default]
    Normal,
    /// Print extra information to standard output.
    ///
    /// String representation: `verbose`.
    Verbose,
}

impl Display for Verbosity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Quiet => write!(f, "quiet"),
            Self::NoWarnings => write!(f, "no-warnings"),
            Self::Normal => write!(f, "normal"),
            Self::Verbose => write!(f, "verbose"),
        }
    }
}

impl Verbosity {
    /// Get the verbosity level from the given environment variable.
    ///
    /// Environment variable value is decoding using [`Verbosity::from_str`].
    /// See [`Verbosity`] variants documentation for valid values.
    pub fn from_env_var(env_var_name: &str) -> Result<Self> {
        let env_var = env::var(env_var_name)?;
        Self::from_str(&env_var, true).map_err(|e| anyhow::anyhow!(e.to_string()))
    }

    /// Check if the verbosity level is the default one.
    pub fn is_default(&self) -> bool {
        *self == Verbosity::default()
    }
}

#[cfg(test)]
mod tests {
    use std::env;
    use clap::ValueEnum;
    use super::Verbosity;

    #[test]
    fn verbosity_ord() {
        use Verbosity::*;
        assert!(Quiet < NoWarnings);
        assert!(NoWarnings < Normal);
        assert!(Normal < Verbose);
    }

    #[test]
    fn verbosity_from_str() {
        use Verbosity::*;
        assert_eq!(Verbosity::from_str(&Quiet.to_string(), true).unwrap(), Quiet);
        assert_eq!(Verbosity::from_str(&NoWarnings.to_string(), true).unwrap(), NoWarnings);
        assert_eq!(Verbosity::from_str(&Normal.to_string(), true).unwrap(), Normal);
        assert_eq!(Verbosity::from_str(&Verbose.to_string(), true).unwrap(), Verbose);
    }

    #[test]
    fn verbosity_from_env_var() {
        use Verbosity::*;
        unsafe {
            env::set_var("SOME_ENV_VAR", "quiet");
        }
        assert_eq!(Verbosity::from_env_var("SOME_ENV_VAR").unwrap(), Quiet);
        unsafe {
            env::set_var("SOME_ENV_VAR", "verbose");
        }
        assert_eq!(Verbosity::from_env_var("SOME_ENV_VAR").unwrap(), Verbose);
        unsafe {
            env::set_var("SOME_ENV_VAR", "no-warnings");
        }
        assert_eq!(Verbosity::from_env_var("SOME_ENV_VAR").unwrap(), NoWarnings);
        assert!(Verbosity::from_env_var("SOME_ENV_VAR_THAT_DOESNT_EXIST").is_err());
    }
}
