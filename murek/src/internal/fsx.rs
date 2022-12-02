//! [`std::fs`] extensions with extra error messaging.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

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
