//! Mostly [`std::fs`] extensions with extra error messaging.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use once_cell::sync::OnceCell;
use tracing::trace;

use create_output_dir::create_output_dir;

/// Equivalent to [`std::fs::canonicalize`] with better error messages.
pub fn canonicalize(p: impl AsRef<Path>) -> Result<PathBuf> {
    _canonicalize(p.as_ref())
}

fn _canonicalize(p: &Path) -> Result<PathBuf> {
    fs::canonicalize(p).with_context(|| format!("failed to get absolute path of `{}`", p.display()))
}

/// Equivalent to [`std::fs::create_dir_all`] with better error messages.
pub fn create_dir_all(p: impl AsRef<Path>) -> Result<()> {
    _create_dir_all(p.as_ref())
}

fn _create_dir_all(p: &Path) -> Result<()> {
    fs::create_dir_all(p)
        .with_context(|| format!("failed to create directory `{}`", p.display()))?;
    Ok(())
}

/// Equivalent to [`std::fs::remove_dir_all`] with better error messages.
pub fn remove_dir_all(p: impl AsRef<Path>) -> Result<()> {
    _remove_dir_all(p.as_ref())
}

fn _remove_dir_all(p: &Path) -> Result<()> {
    fs::remove_dir_all(p)
        .with_context(|| format!("failed to remove directory `{}`", p.display()))?;
    Ok(())
}

#[derive(Copy, Clone, Debug, Default)]
pub struct GuardedExistedPathBufOpts {
    pub is_output_dir: bool,
}

#[derive(Debug)]
pub struct GuardedExistedPathBuf<'p> {
    path: PathBuf,
    opts: GuardedExistedPathBufOpts,
    creation_lock: OnceCell<()>,
    parent: Option<&'p GuardedExistedPathBuf<'p>>,
}

impl<'p> GuardedExistedPathBuf<'p> {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self::with_options(path, Default::default())
    }

    pub fn with_options(path: impl Into<PathBuf>, opts: GuardedExistedPathBufOpts) -> Self {
        Self {
            path: path.into(),
            opts,
            creation_lock: OnceCell::new(),
            parent: None,
        }
    }

    pub fn child(&'p self, path: impl AsRef<Path>) -> Self {
        Self {
            path: self.path.join(path),
            opts: self.opts,
            creation_lock: OnceCell::new(),
            parent: Some(self),
        }
    }

    pub fn as_unchecked(&self) -> &Path {
        &self.path
    }

    pub fn into_unchecked(self) -> PathBuf {
        self.path
    }

    pub fn as_existent(&self) -> Result<&Path> {
        self.ensure_created()?;
        Ok(&self.path)
    }

    pub fn into_existent(self) -> Result<PathBuf> {
        self.ensure_created()?;
        Ok(self.path)
    }

    fn ensure_created(&self) -> Result<()> {
        if let Some(parent) = self.parent {
            parent.ensure_created()?;
        }

        self.creation_lock
            .get_or_try_init(|| {
                trace!(
                    "creating directory {}; output_dir={}",
                    &self.path.display(),
                    self.opts.is_output_dir
                );

                if self.opts.is_output_dir {
                    create_output_dir(&self.path)
                } else {
                    create_dir_all(&self.path)
                }
            })
            .copied()
    }
}
