#![allow(dyn_drop)]

use crate::cargo::cargo_bin;
use assert_fs::TempDir;
use assert_fs::prelude::*;
use serde::de::DeserializeOwned;
use snapbox::cmd::{Command as SnapboxCommand, OutputAssert};
use std::ffi::{OsStr, OsString};
use std::fs;
use std::io::BufRead;
use std::path::{Path, PathBuf};
use std::process::Command as StdCommand;
use std::sync::LazyLock;

#[cfg(feature = "scarb-config")]
use camino::Utf8Path;

pub struct Scarb {
    cache: EnvPath,
    config: EnvPath,
    target: Option<PathBuf>,
    log: OsString,
    scarb_bin: PathBuf,
}

impl Scarb {
    pub fn new() -> Self {
        Self {
            cache: EnvPath::temp_dir(),
            config: EnvPath::temp_dir(),
            target: None,
            log: "scarb=trace".into(),
            scarb_bin: cargo_bin("scarb"),
        }
    }

    #[cfg(feature = "scarb-config")]
    pub fn from_config(config: &scarb::core::Config) -> Self {
        Self {
            cache: EnvPath::borrow(config.dirs().cache_dir.path_unchecked().as_std_path()),
            config: EnvPath::borrow(config.dirs().config_dir.path_unchecked().as_std_path()),
            target: config
                .target_dir_override()
                .map(|p| p.as_std_path().to_path_buf()),
            log: config.log_filter_directive().to_os_string(),
            scarb_bin: cargo_bin("scarb"),
        }
    }

    pub fn quick_snapbox() -> ScarbCommand {
        Self::new().snapbox()
    }

    pub fn snapbox(self) -> ScarbCommand {
        let inner = SnapboxCommand::from_std(self.std_unchecked());
        let state: Vec<Box<dyn Drop>> = vec![Box::new(self.cache), Box::new(self.config)];
        ScarbCommand { inner, state }
    }

    pub fn std(&self) -> StdCommand {
        assert!(
            matches!(self.config, EnvPath::Unmanaged(_)),
            "You must set config directory manually with `config` method to use `std()` command."
        );
        assert!(
            matches!(self.cache, EnvPath::Unmanaged(_)),
            "You must set cache directory manually with `cache` method to use `std()` command."
        );
        self.std_unchecked()
    }

    fn std_unchecked(&self) -> StdCommand {
        let mut cmd = StdCommand::new(self.scarb_bin.clone());
        cmd.env("SCARB_LOG", self.log.clone());
        cmd.env("SCARB_CACHE", self.cache.path());
        cmd.env("SCARB_CONFIG", self.config.path());
        cmd.env("SCARB_INIT_TEST_RUNNER", "none");
        if let Some(target) = self.target.as_ref() {
            cmd.env("SCARB_TARGET_DIR", target);
        }
        cmd
    }

    pub fn isolate_from_extensions(self) -> Self {
        // NOTE: We keep TempDir instance in static, so that it'll be dropped when program ends.
        static ISOLATE: LazyLock<(PathBuf, TempDir)> = LazyLock::new(|| {
            let t = TempDir::new().unwrap();
            let source_bin = cargo_bin("scarb");
            let output_bin = t.child(source_bin.file_name().unwrap()).to_path_buf();
            fs::copy(source_bin, &output_bin).unwrap();
            (output_bin, t)
        });

        Self {
            scarb_bin: ISOLATE.0.clone(),
            ..self
        }
    }

    #[cfg(feature = "scarb-config")]
    pub fn test_config(
        manifest: impl crate::fsx::AssertFsUtf8Ext,
        cache_dir: &Utf8Path,
        config_dir: &Utf8Path,
    ) -> scarb::core::Config {
        scarb::core::Config::builder(manifest.utf8_path())
            .global_cache_dir_override(Some(cache_dir))
            .global_config_dir_override(Some(config_dir))
            .path_env_override(Some(std::iter::empty::<PathBuf>()))
            .ui_verbosity(scarb_ui::Verbosity::Verbose)
            .log_filter_directive(Some("scarb=trace"))
            .build()
            .unwrap()
    }

    pub fn cache(mut self, path: &Path) -> Self {
        self.cache = EnvPath::borrow(path);
        self
    }

    pub fn config(mut self, path: &Path) -> Self {
        self.config = EnvPath::borrow(path);
        self
    }

    pub fn target_dir(mut self, path: &Path) -> Self {
        self.target = Some(path.into());
        self
    }
}

impl Default for Scarb {
    fn default() -> Self {
        Self::new()
    }
}

pub struct ScarbCommand {
    inner: SnapboxCommand,
    state: Vec<Box<dyn Drop>>,
}

impl ScarbCommand {
    pub fn arg(mut self, arg: impl AsRef<OsStr>) -> Self {
        self.inner = self.inner.arg(arg);
        self
    }

    pub fn args(mut self, args: impl IntoIterator<Item = impl AsRef<OsStr>>) -> Self {
        self.inner = self.inner.args(args);
        self
    }

    pub fn env_remove(mut self, key: impl AsRef<OsStr>) -> Self {
        self.inner = self.inner.env_remove(key);
        self
    }

    pub fn env(mut self, key: impl AsRef<OsStr>, value: impl AsRef<OsStr>) -> Self {
        self.inner = self.inner.env(key, value);
        self
    }

    pub fn envs(
        mut self,
        vars: impl IntoIterator<Item = (impl AsRef<OsStr>, impl AsRef<OsStr>)>,
    ) -> Self {
        self.inner = self.inner.envs(vars);
        self
    }

    pub fn current_dir(self, dir: impl AsRef<Path>) -> Self {
        Self {
            state: self.state,
            inner: self.inner.current_dir(dir),
        }
    }

    pub fn assert(self) -> OutputAssert {
        let Self {
            // will be dropped at the end of the block, after `assert` is called
            state: _managed_paths,
            inner,
        } = self;
        inner.assert()
    }

    pub fn timeout(mut self, timeout: std::time::Duration) -> Self {
        self.inner = self.inner.timeout(timeout);
        self
    }

    pub fn output(self) -> Result<std::process::Output, std::io::Error> {
        let Self {
            // will be dropped at the end of the block, after `output` is called
            state: _managed_paths,
            inner,
        } = self;
        inner.output()
    }
}

#[allow(dead_code)]
enum EnvPath {
    Managed(TempDir),
    Unmanaged(PathBuf),
}

impl EnvPath {
    fn temp_dir() -> Self {
        Self::Managed(TempDir::new().unwrap())
    }

    fn borrow(path: impl AsRef<Path>) -> Self {
        Self::Unmanaged(path.as_ref().to_path_buf())
    }

    fn path(&self) -> &Path {
        match self {
            EnvPath::Managed(t) => t.path(),
            EnvPath::Unmanaged(p) => p,
        }
    }
}

impl Drop for EnvPath {
    fn drop(&mut self) {}
}

pub trait CommandExt {
    fn stdout_json<T: DeserializeOwned>(self) -> T;
}

impl CommandExt for SnapboxCommand {
    fn stdout_json<T: DeserializeOwned>(self) -> T {
        let output = self.output().expect("Failed to spawn command");
        assert!(
            output.status.success(),
            "Command failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        for line in BufRead::split(output.stdout.as_slice(), b'\n') {
            let line = line.expect("Failed to read line from stdout");
            match serde_json::de::from_slice::<T>(&line) {
                Ok(t) => return t,
                Err(_) => continue,
            }
        }
        // help: make sure that the command outputs NDJSON (`--json` flag).
        panic!("Failed to deserialize stdout to JSON");
    }
}

impl CommandExt for ScarbCommand {
    fn stdout_json<T: DeserializeOwned>(self) -> T {
        let Self {
            // will be dropped at the end of the block, after `stdout_json` is called
            state: _managed_paths,
            inner,
        } = self;
        inner.stdout_json()
    }
}
