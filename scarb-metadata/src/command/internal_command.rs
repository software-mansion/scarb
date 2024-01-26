use std::collections::HashMap;
use std::env;
use std::ffi::{OsStr, OsString};
use std::path::PathBuf;
use std::process::{Command, Stdio};

/// A builder for `scarb` command invocation.
#[derive(Clone, Debug, Default)]
pub struct InternalScarbCommandBuilder {
    args: Vec<OsString>,
    current_dir: Option<PathBuf>,
    env: HashMap<OsString, Option<OsString>>,
    env_clear: bool,
    inherit_stderr: bool,
    inherit_stdout: bool,
    manifest_path: Option<PathBuf>,
    scarb_path: Option<PathBuf>,
}

impl InternalScarbCommandBuilder {
    /// Creates a default `scarb` command, which will look for `scarb` in `$PATH` and
    /// for `Scarb.toml` in the current directory or its ancestors.
    pub fn new() -> Self {
        Self::default()
    }

    /// Path to `scarb` executable.
    ///
    /// If not set, this will use the `$SCARB` environment variable, and if that is not set, it
    /// will simply be `scarb` and the system will look it up in `$PATH`.
    pub fn scarb_path(&mut self, path: impl Into<PathBuf>) -> &mut Self {
        self.scarb_path = Some(path.into());
        self
    }

    /// Path to `Scarb.toml`.
    ///
    /// If not set, this will look for `Scarb.toml` in the current directory or its ancestors.
    pub fn manifest_path(&mut self, path: impl Into<PathBuf>) -> &mut Self {
        self.manifest_path = Some(path.into());
        self
    }

    /// Current directory of the `scarb` process.
    pub fn current_dir(&mut self, path: impl Into<PathBuf>) -> &mut Self {
        self.current_dir = Some(path.into());
        self
    }

    /// Adds an argument to pass to `scarb`.
    pub fn arg<S: AsRef<OsStr>>(&mut self, arg: S) -> &mut Self {
        self.args.push(arg.as_ref().to_os_string());
        self
    }

    /// Adds multiple arguments to pass to `scarb`.
    pub fn args<I, S>(&mut self, args: I) -> &mut Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        self.args
            .extend(args.into_iter().map(|s| s.as_ref().to_os_string()));
        self
    }

    /// Inserts or updates an environment variable mapping.
    pub fn env(&mut self, key: impl AsRef<OsStr>, val: impl AsRef<OsStr>) -> &mut Self {
        self.env.insert(
            key.as_ref().to_os_string(),
            Some(val.as_ref().to_os_string()),
        );
        self
    }

    /// Adds or updates multiple environment variable mappings.
    pub fn envs<I, K, V>(&mut self, vars: I) -> &mut Self
    where
        I: IntoIterator<Item = (K, V)>,
        K: AsRef<OsStr>,
        V: AsRef<OsStr>,
    {
        for (ref key, ref val) in vars {
            self.env(key, val);
        }
        self
    }

    /// Removes an environment variable mapping.
    pub fn env_remove(&mut self, key: impl AsRef<OsStr>) -> &mut Self {
        let key = key.as_ref();
        if self.env_clear {
            self.env.remove(key);
        } else {
            self.env.insert(key.to_os_string(), None);
        }
        self
    }

    /// Clears the entire environment map for the child process.
    pub fn env_clear(&mut self) -> &mut Self {
        self.env.clear();
        self.env_clear = true;
        self
    }

    /// Inherit standard error, i.e. show Scarb errors in this process's standard error.
    pub fn inherit_stderr(&mut self) -> &mut Self {
        self.inherit_stderr = true;
        self
    }

    /// Inherit standard output, i.e. show Scarb output in this process's standard output.
    pub fn inherit_stdout(&mut self) -> &mut Self {
        self.inherit_stdout = true;
        self
    }

    /// Build executable `scarb` command.
    pub fn command(&self) -> Command {
        let scarb = self
            .scarb_path
            .clone()
            .or_else(|| env::var("SCARB").map(PathBuf::from).ok())
            .unwrap_or_else(|| PathBuf::from("scarb"));

        let mut cmd = Command::new(scarb);

        if let Some(manifest_path) = &self.manifest_path {
            cmd.arg("--manifest-path").arg(manifest_path);
        }

        cmd.args(&self.args);

        if let Some(path) = &self.current_dir {
            cmd.current_dir(path);
        }

        for (key, val) in &self.env {
            if let Some(val) = val {
                cmd.env(key, val);
            } else {
                cmd.env_remove(key);
            }
        }

        if self.env_clear {
            cmd.env_clear();
        }

        if self.inherit_stderr {
            cmd.stderr(Stdio::inherit());
        }

        if self.inherit_stdout {
            cmd.stdout(Stdio::inherit());
        }

        cmd
    }
}
