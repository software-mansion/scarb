//! Mostly [`fs`] extensions with extra error messaging.

use std::fs;
use std::fs::File;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, Context, Result};
use camino::{Utf8Path, Utf8PathBuf};

/// Equivalent to [`fs::canonicalize`] with better error messages.
///
/// Uses [`dunce`] to generate more familiar paths on Windows.
pub fn canonicalize(p: impl AsRef<Path>) -> Result<PathBuf> {
    return inner(p.as_ref());

    fn inner(p: &Path) -> Result<PathBuf> {
        dunce::canonicalize(p)
            .with_context(|| format!("failed to get absolute path of `{}`", p.display()))
    }
}

/// Equivalent to [`fs::canonicalize`], but for Utf-8 paths, with better error messages.
pub fn canonicalize_utf8(p: impl AsRef<Path>) -> Result<Utf8PathBuf> {
    canonicalize(p)?.try_into_utf8()
}

/// Equivalent to [`fs::create_dir_all`] with better error messages.
pub fn create_dir_all(p: impl AsRef<Path>) -> Result<()> {
    return inner(p.as_ref());

    fn inner(p: &Path) -> Result<()> {
        fs::create_dir_all(p)
            .with_context(|| format!("failed to create directory `{}`", p.display()))?;
        Ok(())
    }
}

/// Equivalent to [`fs::remove_dir_all`] with better error messages.
pub fn remove_dir_all(p: impl AsRef<Path>) -> Result<()> {
    return inner(p.as_ref());

    fn inner(p: &Path) -> Result<()> {
        fs::remove_dir_all(p)
            .with_context(|| format!("failed to remove directory `{}`", p.display()))?;
        Ok(())
    }
}

/// Equivalent to [`fs::write`] with better error messages.
pub fn write(path: impl AsRef<Path>, contents: impl AsRef<[u8]>) -> Result<()> {
    return inner(path.as_ref(), contents.as_ref());

    fn inner(path: &Path, contents: &[u8]) -> Result<()> {
        fs::write(path, contents).with_context(|| format!("failed to write `{}`", path.display()))
    }
}

/// Equivalent to [`File::open`] with better error messages.
pub fn open(path: impl AsRef<Path>) -> Result<File> {
    return inner(path.as_ref());

    fn inner(path: &Path) -> Result<File> {
        File::open(path).with_context(|| format!("failed to open `{}`", path.display()))
    }
}

/// Equivalent to [`File::create`] with better error messages.
pub fn create(path: impl AsRef<Path>) -> Result<File> {
    return inner(path.as_ref());

    fn inner(path: &Path) -> Result<File> {
        File::create(path).with_context(|| format!("failed to create `{}`", path.display()))
    }
}

/// Equivalent to [`fs::read`] with better error messages.
pub fn read(path: impl AsRef<Path>) -> Result<Vec<u8>> {
    return inner(path.as_ref());

    fn inner(path: &Path) -> Result<Vec<u8>> {
        fs::read(path).with_context(|| format!("failed to read `{}`", path.display()))
    }
}

/// Equivalent to [`fs::read_to_string`] with better error messages.
pub fn read_to_string(path: impl AsRef<Path>) -> Result<String> {
    return inner(path.as_ref());

    fn inner(path: &Path) -> Result<String> {
        fs::read_to_string(path).with_context(|| format!("failed to read `{}`", path.display()))
    }
}

/// Equivalent to [`fs::rename`] with better error messages.
pub fn rename(from: impl AsRef<Path>, to: impl AsRef<Path>) -> Result<()> {
    return inner(from.as_ref(), to.as_ref());

    fn inner(from: &Path, to: &Path) -> Result<()> {
        fs::rename(from, to).with_context(|| format!("failed to rename file: {}", from.display()))
    }
}

/// Equivalent to [`fs::copy`] with better error messages.
pub fn copy(from: impl AsRef<Path>, to: impl AsRef<Path>) -> Result<u64> {
    return inner(from.as_ref(), to.as_ref());

    fn inner(from: &Path, to: &Path) -> Result<u64> {
        fs::copy(from, to)
            .with_context(|| format!("failed to copy file {} to {}", from.display(), to.display()))
    }
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
