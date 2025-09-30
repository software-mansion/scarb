use assert_fs::TempDir;
use assert_fs::prelude::*;
use fs_extra::dir::{CopyOptions, copy};
use indoc::indoc;
use serde::de::DeserializeOwned;
use snapbox::cmd::Command as SnapboxCommand;
use std::ffi::OsString;
use std::fs;
use std::io::BufRead;
use std::path::{Path, PathBuf};
use std::process::Command as StdCommand;
use std::sync::LazyLock;

use crate::cargo::cargo_bin;

/// Pre-warmed cache directory for test optimization.
/// This static holds a compiled scarb cache with the core library already compiled.
/// When tests run, they copy from this pre-warmed cache instead of recompiling core every time.
static PREWARMED_CACHE: LazyLock<Option<(PathBuf, TempDir)>> = LazyLock::new(|| {
    use assert_fs::fixture::PathChild;
    
    // Create a temporary directory that will live for the entire test run
    let temp_dir = TempDir::new().ok()?;
    
    // Create a basic project with a unique name
    let project_dir = temp_dir.child("this_is_a_cache_warmup");
    project_dir.create_dir_all().ok()?;
    
    // Create Scarb.toml
    project_dir.child("Scarb.toml")
        .write_str(indoc! {r#"
            [package]
            name = "this_is_a_cache_warmup"
            version = "1.0.0"
            edition = "2024_07"

            [dependencies]
        "#})
        .ok()?;
    
    // Create lib.cairo
    project_dir.child("src/lib.cairo")
        .write_str("fn warmup() -> felt252 { 42 }")
        .ok()?;
    
    // Create cache directory
    let cache_dir = temp_dir.child("cache");
    cache_dir.create_dir_all().ok()?;
    
    // Compile the project to warm up the cache
    let scarb_bin = cargo_bin("scarb");
    let output = StdCommand::new(&scarb_bin)
        .arg("build")
        .current_dir(project_dir.path())
        .env("SCARB_CACHE", cache_dir.path())
        .env("SCARB_LOG", "scarb=warn")  // Reduce noise during warmup
        .output()
        .ok()?;
    
    if !output.status.success() {
        eprintln!("WARN: cache warmup build failed");
        eprintln!("stdout: {}", String::from_utf8_lossy(&output.stdout));
        eprintln!("stderr: {}", String::from_utf8_lossy(&output.stderr));
        return None;
    }
    
    // Return cache path and keep temp_dir alive
    Some((cache_dir.to_path_buf(), temp_dir))
});


pub struct Scarb {
    cache: EnvPath,
    config: EnvPath,
    log: OsString,
    scarb_bin: PathBuf,
}

impl Scarb {
    pub fn new() -> Self {
        Self {
            cache: EnvPath::temp_cache_dir(),
            config: EnvPath::temp_dir(),
            log: "scarb=trace".into(),
            scarb_bin: cargo_bin("scarb"),
        }
    }

    #[cfg(feature = "scarb-config")]
    pub fn from_config(config: &scarb::core::Config) -> Self {
        Self {
            cache: EnvPath::borrow(config.dirs().cache_dir.path_unchecked().as_std_path()),
            config: EnvPath::borrow(config.dirs().config_dir.path_unchecked().as_std_path()),
            log: config.log_filter_directive().to_os_string(),
            scarb_bin: cargo_bin("scarb"),
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
        cmd.env("SCARB_INIT_TEST_RUNNER", "cairo-test");
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
}

impl Default for Scarb {
    fn default() -> Self {
        Self::new()
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
    
    /// Creates a temporary cache directory pre-populated with the warmed cache.
    fn temp_cache_dir() -> Self {
        let temp_dir = TempDir::new().unwrap();
        
        // Copy pre-warmed cache contents to the new temp directory if available
        if let Some((prewarmed_cache, _)) = &*PREWARMED_CACHE {
            let mut options = CopyOptions::new();
            options.content_only = true;
            options.overwrite = true;
            
            if let Err(e) = copy(prewarmed_cache, temp_dir.path(), &options) {
                eprintln!("WARN: failed to copy pre-warmed cache: {e}");
                // Continue anyway - the test will just recompile core
            }
        }
        
        Self::Managed(temp_dir)
    }

    #[cfg(feature = "scarb-config")]
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

pub trait ScarbSnapboxExt {
    fn scarb_cache(self, path: impl AsRef<Path>) -> SnapboxCommand;
}

impl ScarbSnapboxExt for SnapboxCommand {
    fn scarb_cache(self, path: impl AsRef<Path>) -> SnapboxCommand {
        self.env("SCARB_CACHE", path.as_ref())
    }
}
