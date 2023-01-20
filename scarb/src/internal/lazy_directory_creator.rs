use std::fmt;

use camino::{Utf8Path, Utf8PathBuf};
use once_cell::sync::OnceCell;
use tracing::trace;

use crate::internal::fsx;

#[derive(Debug)]
pub struct LazyDirectoryCreator<'p> {
    path: Utf8PathBuf,
    creation_lock: OnceCell<()>,
    parent: Option<&'p LazyDirectoryCreator<'p>>,
    is_output_dir: bool,
}

impl<'p> LazyDirectoryCreator<'p> {
    pub fn new(path: impl Into<Utf8PathBuf>) -> Self {
        Self {
            path: path.into(),
            creation_lock: OnceCell::new(),
            parent: None,
            is_output_dir: false,
        }
    }

    pub fn new_output_dir(path: impl Into<Utf8PathBuf>) -> Self {
        Self {
            is_output_dir: true,
            ..Self::new(path)
        }
    }

    pub fn child(&'p self, path: impl AsRef<Utf8Path>) -> Self {
        Self {
            path: self.path.join(path),
            creation_lock: OnceCell::new(),
            parent: Some(self),
            is_output_dir: false,
        }
    }

    pub fn as_unchecked(&self) -> &Utf8Path {
        &self.path
    }

    pub fn as_existent(&self) -> anyhow::Result<&Utf8Path> {
        self.ensure_created()?;
        Ok(&self.path)
    }

    fn ensure_created(&self) -> anyhow::Result<()> {
        if let Some(parent) = self.parent {
            parent.ensure_created()?;
        }

        self.creation_lock
            .get_or_try_init(|| {
                trace!(
                    "creating directory {}; output_dir={}",
                    &self.path,
                    self.is_output_dir
                );

                if self.is_output_dir {
                    create_output_dir::create_output_dir(self.path.as_std_path())
                } else {
                    fsx::create_dir_all(&self.path)
                }
            })
            .copied()
    }
}

impl<'p> fmt::Display for LazyDirectoryCreator<'p> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_unchecked())
    }
}
