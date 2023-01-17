use std::env;
use std::ffi::OsString;
use std::fmt;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, Result};
use directories::ProjectDirs;

#[derive(Debug)]
#[non_exhaustive]
pub struct AppDirs {
    pub cache_dir: PathBuf,
    pub config_dir: PathBuf,
    pub path_dirs: Vec<PathBuf>,
}

impl AppDirs {
    pub fn std() -> Result<Self> {
        let pd = ProjectDirs::from("co", "starkware", "scarb").ok_or_else(|| {
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

        Ok(Self {
            cache_dir: pd.cache_dir().into(),
            config_dir: pd.config_dir().into(),
            path_dirs,
        })
    }

    pub fn apply_env_overrides(&mut self) -> Result<()> {
        if let Some(path) = env::var_os("SCARB_CACHE") {
            self.cache_dir = PathBuf::from(path);
        }

        if let Some(path) = env::var_os("SCARB_CONFIG") {
            self.config_dir = PathBuf::from(path);
        }

        Ok(())
    }

    pub fn path_env(&self) -> OsString {
        env::join_paths(self.path_dirs.iter()).unwrap()
    }

    pub fn registry_dir(&self, category: impl AsRef<Path>) -> PathBuf {
        self.cache_dir.join("registry").join(category)
    }
}

impl fmt::Display for AppDirs {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "cache dir:  {}", self.cache_dir.display())?;
        writeln!(f, "config dir: {}", self.config_dir.display())?;
        writeln!(f, "PATH:       {}", self.path_env().to_string_lossy())?;
        Ok(())
    }
}
