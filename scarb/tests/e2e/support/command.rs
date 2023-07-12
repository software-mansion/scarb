use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::Command as StdCommand;
use std::{fs, iter};

use assert_fs::TempDir;
use once_cell::sync::Lazy;
use serde::de::DeserializeOwned;
use snapbox::cmd::cargo_bin;
use snapbox::cmd::Command as SnapboxCommand;

use scarb::core::Config;
use scarb::ui::Verbosity;

use crate::support::fsx::{AssertFsUtf8Ext, PathUtf8Ext};

pub struct Scarb {
    cache: EnvPath,
    config: EnvPath,
    log: OsString,
    scarb_bin: &'static Path,
}

impl Scarb {
    pub fn new() -> Self {
        Self {
            cache: EnvPath::temp_dir(),
            config: EnvPath::temp_dir(),
            log: "scarb=trace".into(),
            scarb_bin: cargo_bin!("scarb"),
        }
    }

    pub fn from_config(config: &Config) -> Self {
        Self {
            cache: EnvPath::borrow(config.dirs().cache_dir.path_unchecked().as_std_path()),
            config: EnvPath::borrow(config.dirs().config_dir.path_unchecked().as_std_path()),
            log: config.log_filter_directive().to_os_string(),
            scarb_bin: cargo_bin!("scarb"),
        }
    }

    pub fn quick_snapbox() -> SnapboxCommand {
        Self::new().snapbox()
    }

    pub fn snapbox(self) -> SnapboxCommand {
        SnapboxCommand::from_std(self.std())
    }

    pub fn std(self) -> StdCommand {
        let mut cmd = StdCommand::new(self.scarb_bin);
        cmd.env("SCARB_LOG", self.log);
        cmd.env("SCARB_CACHE", self.cache.path());
        cmd.env("SCARB_CONFIG", self.config.path());
        cmd
    }

    pub fn isolate_from_extensions(self) -> Self {
        static ISOLATED_BIN: Lazy<PathBuf> = Lazy::new(|| {
            let source_bin = cargo_bin!("scarb");

            let output_dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("isolated_scarb");
            fs::create_dir_all(&output_dir).unwrap();

            let output_bin = output_dir.join(source_bin.file_name().unwrap());
            fs::copy(source_bin, &output_bin).unwrap();

            output_bin
        });

        Self {
            scarb_bin: &*ISOLATED_BIN,
            ..self
        }
    }

    pub fn test_config(manifest: impl AssertFsUtf8Ext) -> Config {
        let cache_dir = TempDir::new().unwrap();
        let config_dir = TempDir::new().unwrap();

        Config::builder(manifest.utf8_path())
            .global_cache_dir_override(Some(cache_dir.try_as_utf8().unwrap()))
            .global_config_dir_override(Some(config_dir.try_as_utf8().unwrap()))
            .path_env_override(Some(iter::empty::<PathBuf>()))
            .ui_verbosity(Verbosity::Verbose)
            .log_filter_directive(Some("scarb=trace"))
            .build()
            .unwrap()
    }
}

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
        serde_json::de::from_slice(&output.stdout).expect("Failed to deserialize stdout to JSON")
    }
}
