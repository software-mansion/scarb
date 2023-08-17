use std::env;
use std::fmt::Display;
use std::str::FromStr;

use anyhow::bail;

/// The requested verbosity of output.
///
/// # Ordering
/// [`Verbosity::Quiet`] < [`Verbosity::Normal`] < [`Verbosity::Verbose`]
#[derive(Copy, Clone, Debug, Default, Eq, PartialEq, Ord, PartialOrd)]
pub enum Verbosity {
    Quiet,
    #[default]
    Normal,
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

    fn from_str(s: &str) -> anyhow::Result<Self> {
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
    pub fn from_env_var(env_var_name: &str) -> anyhow::Result<Self> {
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

        assert!(matches!(
            Verbosity::from_env_var("SOME_ENV_VAR_THAT_DOESNT_EXIST"),
            Err(_)
        ));
    }
}
