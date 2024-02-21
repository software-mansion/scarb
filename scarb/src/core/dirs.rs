use std::env;
use std::ffi::OsString;
use std::fmt;
use std::path::PathBuf;

use anyhow::{anyhow, Result};
use camino::Utf8PathBuf;
use directories::ProjectDirs;

use crate::flock::Filesystem;
use crate::internal::fsx::PathUtf8Ext;

#[derive(Debug)]
pub struct AppDirs {
    pub cache_dir: Filesystem,
    pub config_dir: Filesystem,
    pub path_dirs: Vec<PathBuf>,
}

impl AppDirs {
    pub(crate) fn init(
        cache_dir_override: Option<Utf8PathBuf>,
        config_dir_override: Option<Utf8PathBuf>,
        path_dirs_override: Option<Vec<PathBuf>>,
    ) -> Result<Self> {
        let pd = ProjectDirs::from("com", "swmansion", "scarb").ok_or_else(|| {
            anyhow!("no valid home directory path could be retrieved from the operating system")
        })?;

        let path_dirs = match path_dirs_override {
            Some(p) => p,
            None => {
                let mut path_dirs = if let Some(val) = env::var_os("PATH") {
                    env::split_paths(&val).collect()
                } else {
                    vec![]
                };

                let home_bin = pd.data_local_dir().join("bin");

                if !path_dirs.iter().any(|p| p == &home_bin) {
                    path_dirs.push(home_bin);
                };

                path_dirs
            }
        };

        let cache_dir = match cache_dir_override {
            Some(p) => p,
            None => pd.cache_dir().try_to_utf8()?,
        };

        let config_dir = match config_dir_override {
            Some(p) => p,
            None => pd.config_dir().try_to_utf8()?,
        };

        Ok(Self {
            cache_dir: Filesystem::new_output_dir(cache_dir),
            config_dir: Filesystem::new(config_dir),
            path_dirs,
        })
    }

    pub fn path_env(&self) -> OsString {
        env::join_paths(self.path_dirs.iter()).unwrap()
    }

    pub fn registry_dir(&self) -> Filesystem {
        self.cache_dir.child("registry")
    }

    pub fn procedural_macros_dir(&self) -> Filesystem {
        self.cache_dir.child("plugins").child("proc_macro")
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
