use anyhow::{Result, bail};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tracing::{debug, trace};

#[derive(Clone, Debug)]
pub struct Assets {
    search_paths: Arc<Vec<PathBuf>>,
}

impl Assets {
    /// Constructs a new assets directory that provides no assets.
    pub(crate) fn empty() -> Self {
        Self {
            search_paths: Default::default(),
        }
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
        debug!(
            name,
            search_paths = ?(
                self.search_paths
                    .iter()
                    .map(|p|p.display())
                    .collect::<Vec<_>>()
            ),
            "looking for asset",
        );
        for p in self.search_paths.iter() {
            let path = p.join(name);
            let exists = path.exists();
            trace!(path=?path.display(), exists=?exists, "trying");
            if exists {
                debug!(path=?path.display(), "found");
                return Ok(path);
            }
        }
        debug!(name, "asset not found");
        bail!("asset not found: {name}")
    }
}
