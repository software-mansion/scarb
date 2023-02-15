use std::env;
use std::ffi::OsString;
use std::fmt;
use std::path::PathBuf;

use anyhow::{anyhow, Result};
use directories::ProjectDirs;

use crate::flock::{Filesystem, RootFilesystem};
use crate::internal::fsx::{PathBufUtf8Ext, PathUtf8Ext};

// TODO(mkaput): Construction needs refinement here.
#[derive(Debug)]
pub struct AppDirs {
    pub cache_dir: RootFilesystem,
    pub config_dir: RootFilesystem,
    pub path_dirs: Vec<PathBuf>,
}

impl AppDirs {
    pub fn std() -> Result<Self> {
        let pd = ProjectDirs::from("com", "swmansion", "scarb").ok_or_else(|| {
            anyhow!("no valid home directory path could be retrieved from the operating system")
        })?;

        let mut path_dirs = if let Some(val) = env::var_os("PATH") {
            env::split_paths(&val).collect()
        } else {
            vec![]
        };

        let home_bin = pd.data_local_dir().join("bin");

        if !path_dirs.iter().any(|p| p == &home_bin) {
            path_dirs.push(home_bin);
        };

        let cache_dir = pd.cache_dir().try_to_utf8()?;
        let config_dir = pd.config_dir().try_to_utf8()?;

        Ok(Self {
            cache_dir: RootFilesystem::new_output_dir(cache_dir),
            config_dir: RootFilesystem::new(config_dir),
            path_dirs,
        })
    }

    pub fn apply_env_overrides(&mut self) -> Result<()> {
        if let Some(path) = env::var_os("SCARB_CACHE") {
            let cache_dir = PathBuf::from(path).try_into_utf8()?;
            self.cache_dir = RootFilesystem::new_output_dir(cache_dir);
        }

        if let Some(path) = env::var_os("SCARB_CONFIG") {
            let config_dir = PathBuf::from(path).try_into_utf8()?;
            self.config_dir = RootFilesystem::new(config_dir);
        }

        Ok(())
    }

    pub fn path_env(&self) -> OsString {
        env::join_paths(self.path_dirs.iter()).unwrap()
    }

    pub fn registry_dir(&self) -> Filesystem<'_> {
        self.cache_dir.child("registry")
    }
}

impl fmt::Display for AppDirs {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "cache dir:  {}", self.cache_dir)?;
        writeln!(f, "config dir: {}", self.config_dir)?;
        writeln!(f, "PATH:       {}", self.path_env().to_string_lossy())?;
        Ok(())
    }
}
