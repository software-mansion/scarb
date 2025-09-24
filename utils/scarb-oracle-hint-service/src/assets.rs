use anyhow::{Result, anyhow, bail};
use std::path::{Path, PathBuf};
use std::sync::Arc;

#[derive(Clone, Debug, Default)]
pub struct Assets {
    search_paths: Arc<Vec<PathBuf>>,
}

impl Assets {
    /// Constructs a new assets directory that provides no assets.
    pub(crate) fn new() -> Self {
        Default::default()
    }

    /// Constructs a new assets directory that looks for assets related to the given Cairo
    /// executable.
    ///
    /// ## Panics
    /// The provided path must be a path to a **file**.
    pub(crate) fn for_executable(path: &Path) -> Self {
        let base_dir = path
            .parent()
            .unwrap_or_else(|| panic!("path is not a file: {}", path.display()));
        Self {
            search_paths: Arc::new(vec![base_dir.into()]),
        }
    }

    /// Looks for an asset with the given name in this directory and returns its path.
    ///
    /// If the asset is not found, this function returns an error.
    pub fn fetch(&self, name: &str) -> Result<PathBuf> {
        self.search_paths
            .iter()
            .map(|p| p.join(name))
            .find(|p| p.exists())
            .ok_or_else(|| anyhow!("asset not found: {name}"))
    }
}
