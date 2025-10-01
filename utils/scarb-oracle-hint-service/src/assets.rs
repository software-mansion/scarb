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
        let safe_name = normalize_lexically(Path::new(name))?;
        for p in self.search_paths.iter() {
            let path = p.join(&safe_name);
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

/// Normalize a path, including `..` without traversing the filesystem.
/// Returns an error if normalization would leave leading `..` components.
/// This is unstable in Rust's std.
pub fn normalize_lexically(path: &Path) -> Result<PathBuf> {
    use std::path::Component;

    let err = || {
        bail!(
            "invalid asset path `{path}`: parent reference `..` points outside of base directory",
            path = path.display()
        )
    };

    let mut lexical = PathBuf::new();
    let mut iter = path.components().peekable();

    // Find the root, if any, and add it to the lexical path.
    // Here we treat the Windows path "C:\" as a single "root" even though
    // `components` splits it into two: (Prefix, RootDir).
    let root = match iter.peek() {
        Some(Component::ParentDir) => return err(),
        Some(p @ Component::RootDir) | Some(p @ Component::CurDir) => {
            lexical.push(p);
            iter.next();
            lexical.as_os_str().len()
        }
        Some(Component::Prefix(prefix)) => {
            lexical.push(prefix.as_os_str());
            iter.next();
            if let Some(p @ Component::RootDir) = iter.peek() {
                lexical.push(p);
                iter.next();
            }
            lexical.as_os_str().len()
        }
        None => return Ok(PathBuf::new()),
        Some(Component::Normal(_)) => 0,
    };

    for component in iter {
        match component {
            Component::RootDir => unreachable!(),
            Component::Prefix(_) => return err(),
            Component::CurDir => continue,
            Component::ParentDir => {
                // It's an error if ParentDir causes us to go above the "root".
                if lexical.as_os_str().len() == root {
                    return err();
                } else {
                    lexical.pop();
                }
            }
            Component::Normal(path) => lexical.push(path),
        }
    }
    Ok(lexical)
}
