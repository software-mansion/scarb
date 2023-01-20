use std::path::{Path, PathBuf};

use once_cell::sync::OnceCell;
use tracing::trace;

use crate::internal::fsx;

#[derive(Debug)]
pub struct LazyDirectoryCreator<'p> {
    path: PathBuf,
    creation_lock: OnceCell<()>,
    parent: Option<&'p LazyDirectoryCreator<'p>>,
    is_output_dir: bool,
}

impl<'p> LazyDirectoryCreator<'p> {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            creation_lock: OnceCell::new(),
            parent: None,
            is_output_dir: false,
        }
    }

    pub fn new_output_dir(path: impl Into<PathBuf>) -> Self {
        Self {
            is_output_dir: true,
            ..Self::new(path)
        }
    }

    pub fn child(&'p self, path: impl AsRef<Path>) -> Self {
        Self {
            path: self.path.join(path),
            creation_lock: OnceCell::new(),
            parent: Some(self),
            is_output_dir: false,
        }
    }

    pub fn as_unchecked(&self) -> &Path {
        &self.path
    }

    pub fn into_unchecked(self) -> PathBuf {
        self.path
    }

    pub fn as_existent(&self) -> anyhow::Result<&Path> {
        self.ensure_created()?;
        Ok(&self.path)
    }

    pub fn into_existent(self) -> anyhow::Result<PathBuf> {
        self.ensure_created()?;
        Ok(self.path)
    }

    fn ensure_created(&self) -> anyhow::Result<()> {
        if let Some(parent) = self.parent {
            parent.ensure_created()?;
        }

        self.creation_lock
            .get_or_try_init(|| {
                trace!(
                    "creating directory {}; output_dir={}",
                    &self.path.display(),
                    self.is_output_dir
                );

                if self.is_output_dir {
                    create_output_dir::create_output_dir(&self.path)
                } else {
                    fsx::create_dir_all(&self.path)
                }
            })
            .copied()
    }
}
