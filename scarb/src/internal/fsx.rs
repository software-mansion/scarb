//! Mostly [`std::fs`] extensions with extra error messaging.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, Context, Result};
use camino::{Utf8Path, Utf8PathBuf};
use once_cell::sync::OnceCell;
use tracing::trace;

use create_output_dir::create_output_dir;

/// Equivalent to [`std::fs::canonicalize`] with better error messages.
pub fn canonicalize(p: impl AsRef<Path>) -> Result<PathBuf> {
    canonicalize_impl(p.as_ref())
}

fn canonicalize_impl(p: &Path) -> Result<PathBuf> {
    fs::canonicalize(p).with_context(|| format!("failed to get absolute path of `{}`", p.display()))
}

/// Equivalent to [`std::fs::create_dir_all`] with better error messages.
pub fn create_dir_all(p: impl AsRef<Path>) -> Result<()> {
    create_dir_all_impl(p.as_ref())
}

fn create_dir_all_impl(p: &Path) -> Result<()> {
    fs::create_dir_all(p)
        .with_context(|| format!("failed to create directory `{}`", p.display()))?;
    Ok(())
}

/// Equivalent to [`std::fs::remove_dir_all`] with better error messages.
pub fn remove_dir_all(p: impl AsRef<Path>) -> Result<()> {
    remove_dir_all_impl(p.as_ref())
}

fn remove_dir_all_impl(p: &Path) -> Result<()> {
    fs::remove_dir_all(p)
        .with_context(|| format!("failed to remove directory `{}`", p.display()))?;
    Ok(())
}

/// Equivalent to [`std::fs::write`] with better error messages.
pub fn write(path: impl AsRef<Path>, contents: impl AsRef<[u8]>) -> Result<()> {
    let path = path.as_ref();
    let contents = contents.as_ref();
    write_impl(path, contents)
}

fn write_impl(path: &Path, contents: &[u8]) -> Result<()> {
    fs::write(path, contents).with_context(|| format!("failed to write `{}`", path.display()))
}

pub trait PathUtf8Ext {
    fn try_as_utf8(&'_ self) -> Result<&'_ Utf8Path>;

    fn try_to_utf8(&self) -> Result<Utf8PathBuf> {
        self.try_as_utf8().map(|p| p.to_path_buf())
    }
}

pub trait PathBufUtf8Ext {
    fn try_into_utf8(self) -> Result<Utf8PathBuf>;
}

impl PathUtf8Ext for Path {
    fn try_as_utf8(&'_ self) -> Result<&'_ Utf8Path> {
        Utf8Path::from_path(self)
            .ok_or_else(|| anyhow!("path `{}` is not UTF-8 encoded", self.display()))
    }
}

impl PathUtf8Ext for PathBuf {
    fn try_as_utf8(&'_ self) -> Result<&'_ Utf8Path> {
        self.as_path().try_as_utf8()
    }
}

impl PathBufUtf8Ext for PathBuf {
    fn try_into_utf8(self) -> Result<Utf8PathBuf> {
        Utf8PathBuf::from_path_buf(self)
            .map_err(|path| anyhow!("path `{}` is not UTF-8 encoded", path.display()))
    }
}

#[derive(Debug)]
pub struct GuardedExistedPathBuf<'p> {
    path: PathBuf,
    creation_lock: OnceCell<()>,
    parent: Option<&'p GuardedExistedPathBuf<'p>>,
    is_output_dir: bool,
}

impl<'p> GuardedExistedPathBuf<'p> {
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
                    self.is_output_dir
                );

                if self.is_output_dir {
                    create_output_dir(&self.path)
                } else {
                    create_dir_all(&self.path)
                }
            })
            .copied()
    }
}
