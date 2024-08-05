//! This module provides reusable [`clap`] arguments for common tasks in Scarb ecosystem.

pub use features::*;
pub use packages_filter::*;
pub use verbosity::*;

mod features;
mod packages_filter;
mod verbosity;

/// This trait can be used to convert CLI argument into a set of environment variables.
///
/// This is useful when you want to pass CLI arguments and pass them to Scarb called in a child process.
pub trait ToEnvVars {
    /// Convert to a set of environment variables.
    fn to_env_vars(self) -> Vec<(String, String)>;
}
