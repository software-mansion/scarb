use crate::cargo::cargo_bin;
use anyhow::Context;
use assert_fs::TempDir;
use assert_fs::prelude::*;
use indoc::indoc;
use serde::de::DeserializeOwned;
use snapbox::cmd::Command as SnapboxCommand;
use std::ffi::OsString;
use std::fs;
use std::io::BufRead;
use std::path::{Path, PathBuf};
use std::process::Command as StdCommand;
use std::sync::LazyLock;

pub struct Scarb {
    cache: EnvPath,
    config: EnvPath,
    target: EnvPath,
    log: OsString,
    scarb_bin: PathBuf,
    incremental: Incremental,
}

impl Scarb {
    pub fn new() -> Self {
        Self {
            cache: EnvPath::Unspecified,
            config: EnvPath::temp_dir(),
            target: EnvPath::Unspecified,
            log: "scarb=trace".into(),
            scarb_bin: cargo_bin("scarb"),
            incremental: Default::default(),
        }
    }

    #[cfg(feature = "scarb-config")]
    pub fn from_config(config: &scarb::core::Config) -> Self {
        Self {
            cache: EnvPath::borrow(config.dirs().cache_dir.path_unchecked().as_std_path()),
            config: EnvPath::borrow(config.dirs().config_dir.path_unchecked().as_std_path()),
            target: config
                .target_dir_override()
                .map(|p| EnvPath::borrow(p.as_std_path()))
                .unwrap_or_default(),
            log: config.log_filter_directive().to_os_string(),
            scarb_bin: cargo_bin("scarb"),
            incremental: Default::default(),
        }
    }

    pub fn quick_snapbox() -> SnapboxCommand {
        Self::new().snapbox()
    }

    pub fn snapbox(self) -> SnapboxCommand {
        SnapboxCommand::from_std(self.std())
    }

    pub fn std(self) -> StdCommand {
        /// This static holds a compiled scarb cache and incremental cache with the core library
        /// already compiled. When tests run in Incremental::Shared mode, this cache is reused
        /// to speed up test execution.
        static SHARED_CACHE: LazyLock<SharedCache> = LazyLock::new(force_warmup_shared_cache);

        let mut cmd = StdCommand::new(self.scarb_bin);

        cmd.env("SCARB_LOG", self.log);

        cmd.env(
            "SCARB_CACHE",
            self.cache
                .path()
                .unwrap_or_else(|| SHARED_CACHE.cache.path()),
        );

        if let Some(config) = self.config.path() {
            cmd.env("SCARB_CONFIG", config);
        }

        cmd.env("SCARB_INIT_TEST_RUNNER", "cairo-test");

        if let Some(target) = self.target.path() {
            cmd.env("SCARB_TARGET_DIR", target);
        }

        cmd.env("SCARB_INCREMENTAL", self.incremental.env());

        if self.incremental == Incremental::Shared {
            cmd.env(
                "__SCARB_INCREMENTAL_BASE_DIR",
                SHARED_CACHE.incremental.path(),
            );
        }

        cmd
    }

    pub fn isolate_from_extensions(self) -> Self {
        // NOTE: We keep the TempDir instance in a static variable
        //   so that it will be dropped when the program ends.
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
    pub fn test_config(manifest: impl crate::fsx::AssertFsUtf8Ext) -> scarb::core::Config {
        use crate::fsx::PathUtf8Ext;

        let cache_dir = TempDir::new().unwrap();
        let config_dir = TempDir::new().unwrap();

        scarb::core::Config::builder(manifest.utf8_path())
            .global_cache_dir_override(Some(cache_dir.try_as_utf8().unwrap()))
            .global_config_dir_override(Some(config_dir.try_as_utf8().unwrap()))
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

    pub fn target_dir(mut self, path: &Path) -> Self {
        self.target = EnvPath::borrow(path);
        self
    }

    pub fn incremental(mut self, incremental: Incremental) -> Self {
        self.incremental = incremental;
        self
    }
}

impl Default for Scarb {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Default)]
enum EnvPath {
    Managed(TempDir),
    Unmanaged(PathBuf),
    #[default]
    Unspecified,
}

impl EnvPath {
    fn temp_dir() -> Self {
        Self::Managed(TempDir::new().unwrap())
    }

    fn borrow(path: impl AsRef<Path>) -> Self {
        Self::Unmanaged(path.as_ref().to_path_buf())
    }

    fn path(&self) -> Option<&Path> {
        match self {
            EnvPath::Managed(t) => Some(t.path()),
            EnvPath::Unmanaged(p) => Some(p),
            EnvPath::Unspecified => None,
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

/// Specifies how scarb incremental compilation should behave in this test.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub enum Incremental {
    // Incremental compilation is disabled, i.e. `SCARB_INCREMENTAL=0` is set.
    No,

    // In isolated mode, incremental compilation is enabled but not shared with other tests.
    Isolated,

    // In shared mode, all tests use the same cache and incremental directories.
    #[default]
    Shared,
}

impl Incremental {
    fn env(self) -> &'static str {
        match self {
            Incremental::No => "0",
            Incremental::Isolated | Incremental::Shared => "1",
        }
    }
}

struct SharedCache {
    cache: TempDir,
    incremental: TempDir,
}

fn force_warmup_shared_cache() -> SharedCache {
    let cache = TempDir::new().unwrap();
    let incremental = TempDir::new().unwrap();
    let work = TempDir::new().unwrap();

    work.child("Scarb.toml")
        .write_str(indoc! {r#"
            [package]
            name = "scarb_e2e_cache_warmup"
            version = "1.0.0"
            edition = "2024_07"
        "#})
        .unwrap();

    work.child("src/lib.cairo")
        .write_str("fn warmup() -> felt252 { 42 }")
        .unwrap();

    let result = StdCommand::new(cargo_bin("scarb"))
        .env("SCARB_LOG", "warn") // Reduce log noise during warmup.
        .env("SCARB_CACHE", cache.path())
        .env("__SCARB_INCREMENTAL_BASE_DIR", incremental.path())
        .arg("build")
        .current_dir(&work)
        .status()
        .context("WARN cache warmup failed");

    match result {
        Ok(status) if status.success() => {}
        Ok(status) => eprintln!("WARN cache warmup build failed, code={status}"),
        Err(e) => eprintln!("{e:?}"),
    }

    SharedCache { cache, incremental }
}
