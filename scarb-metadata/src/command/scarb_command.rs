use std::ffi::OsStr;
use std::io;
use std::path::PathBuf;
use std::process::Output;
use crate::command::internal_command::InternalScarbCommandBuilder;
use thiserror::Error;

/// Error thrown while trying to execute `scarb` command.
#[derive(Error, Debug)]
#[non_exhaustive]
pub enum ScarbCommandError {
    /// Failed to read `scarb` output.
    #[error("failed to read `scarb` output")]
    Io(#[from] io::Error),
    /// Error during execution of `scarb` command.
    #[error("`scarb metadata` exited with error")]
    ScarbError,
}

/// A builder for `scarb` command invocation.
#[derive(Clone, Debug, Default)]
pub struct ScarbCommand {
    inner: InternalScarbCommandBuilder,
    print_stdout: bool,
}

impl ScarbCommand {
    /// Creates a default `scarb` command, which will look for `scarb` in `$PATH` and
    /// for `Scarb.toml` in the current directory or its ancestors.
    pub fn new() -> Self {
        let mut cmd = InternalScarbCommandBuilder::new();
        cmd.inherit_stderr();
        cmd.inherit_stdout();
        Self { 
            inner: cmd,
            print_stdout: false,
        }
    }

    /// Creates a `scarb` command that captures output while still printing it to stdout.
    pub fn new_with_output(print_stdout: bool) -> Self {
        // We can not just use self.inner.inherit_stdout()
        // Because it will make output.stdout empty
        let mut cmd = InternalScarbCommandBuilder::new();
        cmd.inherit_stderr();
        Self { 
            inner: cmd,
            print_stdout,
        }
    }

    /// Path to `scarb` executable.
    ///
    /// If not set, this will use the `$SCARB` environment variable, and if that is not set, it
    /// will simply be `scarb` and the system will look it up in `$PATH`.
    pub fn scarb_path(&mut self, path: impl Into<PathBuf>) -> &mut Self {
        self.inner.scarb_path(path);
        self
    }

    /// Path to `Scarb.toml`.
    ///
    /// If not set, this will look for `Scarb.toml` in the current directory or its ancestors.
    pub fn manifest_path(&mut self, path: impl Into<PathBuf>) -> &mut Self {
        self.inner.manifest_path(path);
        self
    }

    /// Current directory of the `scarb metadata` process.
    pub fn current_dir(&mut self, path: impl Into<PathBuf>) -> &mut Self {
        self.inner.current_dir(path);
        self
    }

    /// Adds an argument to pass to `scarb`.
    pub fn arg<S: AsRef<OsStr>>(&mut self, arg: S) -> &mut Self {
        self.inner.arg(arg);
        self
    }

    /// Adds multiple arguments to pass to `scarb`.
    pub fn args<I, S>(&mut self, args: I) -> &mut Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        self.inner.args(args);
        self
    }

    /// Inserts or updates an environment variable mapping.
    pub fn env(&mut self, key: impl AsRef<OsStr>, val: impl AsRef<OsStr>) -> &mut Self {
        self.inner.env(key, val);
        self
    }

    /// Adds or updates multiple environment variable mappings.
    pub fn envs<I, K, V>(&mut self, vars: I) -> &mut Self
    where
        I: IntoIterator<Item = (K, V)>,
        K: AsRef<OsStr>,
        V: AsRef<OsStr>,
    {
        self.inner.envs(vars);
        self
    }

    /// Removes an environment variable mapping.
    pub fn env_remove(&mut self, key: impl AsRef<OsStr>) -> &mut Self {
        self.inner.env_remove(key);
        self
    }

    /// Clears the entire environment map for the child process.
    pub fn env_clear(&mut self) -> &mut Self {
        self.inner.env_clear();
        self
    }

    /// Runs configured `scarb` command.
    pub fn run(&self) -> Result<(), ScarbCommandError> {
        let mut cmd = self.inner.command();
        if cmd.status()?.success() {
            Ok(())
        } else {
            Err(ScarbCommandError::ScarbError)
        }
    }

    /// Runs configured `scarb` command and returns its output.
    pub fn run_with_output(&self) -> Result<Output, ScarbCommandError> {
        let mut cmd = self.inner.command();
        let output = cmd.output()?;
        
        if self.print_stdout {
            print!("{}", String::from_utf8_lossy(&output.stdout));
        }
        
        if output.status.success() {
            Ok(output)
        } else {
            Err(ScarbCommandError::ScarbError)
        }
    }
}
