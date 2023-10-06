use std::env;
use std::fmt::Display;
use std::str::FromStr;

use anyhow::{bail, Result};

/// The requested verbosity of output.
///
/// # Ordering
/// [`Verbosity::Quiet`] < [`Verbosity::Normal`] < [`Verbosity::Verbose`]
#[derive(Copy, Clone, Debug, Default, Eq, PartialEq, Ord, PartialOrd)]
pub enum Verbosity {
    /// Avoid printing anything to standard output.
    ///
    /// String representation: `quiet`.
    Quiet,
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
            Self::Normal => write!(f, "normal"),
            Self::Verbose => write!(f, "verbose"),
        }
    }
}

impl FromStr for Verbosity {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        match s {
            "quiet" => Ok(Verbosity::Quiet),
            "normal" => Ok(Verbosity::Normal),
            "verbose" => Ok(Verbosity::Verbose),
            "" => bail!("empty string cannot be used as verbosity level"),
            _ => bail!("invalid verbosity level: {s}"),
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
        Self::from_str(env_var.as_str())
    }
}

#[cfg(test)]
mod tests {
    use std::env;

    use super::Verbosity;

    #[test]
    fn verbosity_ord() {
        use Verbosity::*;
        assert!(Quiet < Normal);
        assert!(Normal < Verbose);
    }

    #[test]
    fn verbosity_from_str() {
        use Verbosity::*;
        assert_eq!(Quiet.to_string().parse::<Verbosity>().unwrap(), Quiet);
        assert_eq!(Normal.to_string().parse::<Verbosity>().unwrap(), Normal);
        assert_eq!(Verbose.to_string().parse::<Verbosity>().unwrap(), Verbose);
    }

    #[test]
    fn verbosity_from_env_var() {
        use Verbosity::*;
        env::set_var("SOME_ENV_VAR", "quiet");
        assert_eq!(Verbosity::from_env_var("SOME_ENV_VAR").unwrap(), Quiet);
        env::set_var("SOME_ENV_VAR", "verbose");
        assert_eq!(Verbosity::from_env_var("SOME_ENV_VAR").unwrap(), Verbose);
        assert!(Verbosity::from_env_var("SOME_ENV_VAR_THAT_DOESNT_EXIST").is_err());
    }
}
