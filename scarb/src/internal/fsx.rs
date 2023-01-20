//! Mostly [`std::fs`] extensions with extra error messaging.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, Context, Result};
use camino::{Utf8Path, Utf8PathBuf};

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
