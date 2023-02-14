use std::path::{Path, PathBuf};
use std::process::Command as StdCommand;

use assert_fs::TempDir;
use snapbox::cmd::cargo_bin;
use snapbox::cmd::Command as SnapboxCommand;

use scarb::core::Config;
use scarb::dirs::AppDirs;
use scarb::flock::RootFilesystem;
use scarb::ui::{OutputFormat, Ui, Verbosity};

use crate::support::fsx::{AssertFsUtf8Ext, PathUtf8Ext};

pub struct Scarb {
    cache: EnvPath,
    config: EnvPath,
}

impl Scarb {
    pub fn new() -> Self {
        Self {
            cache: EnvPath::temp_dir(),
            config: EnvPath::temp_dir(),
        }
    }

    pub fn from_config(config: &Config) -> Self {
        Self {
            cache: EnvPath::borrow(config.dirs().cache_dir.path_unchecked().as_std_path()),
            config: EnvPath::borrow(config.dirs().config_dir.path_unchecked().as_std_path()),
        }
    }

    pub fn quick_snapbox() -> SnapboxCommand {
        Self::new().snapbox()
    }

    pub fn snapbox(self) -> SnapboxCommand {
        SnapboxCommand::from_std(self.std())
    }

    pub fn std(self) -> StdCommand {
        let mut cmd = StdCommand::new(cargo_bin!("scarb"));
        cmd.env("SCARB_LOG", "scarb=trace");
        cmd.env("SCARB_CACHE", self.cache.path());
        cmd.env("SCARB_CONFIG", self.config.path());
        cmd
    }

    pub fn test_config(manifest: impl AssertFsUtf8Ext) -> Config {
        let cache_dir = TempDir::new().unwrap();
        let config_dir = TempDir::new().unwrap();

        Config::init(
            manifest.utf8_path().to_path_buf(),
            AppDirs {
                cache_dir: RootFilesystem::new(cache_dir.try_as_utf8().unwrap().to_path_buf()),
                config_dir: RootFilesystem::new(config_dir.try_as_utf8().unwrap().to_path_buf()),
                path_dirs: Vec::new(),
            },
            Ui::new(Verbosity::Verbose, OutputFormat::Text),
        )
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
