use crate::command::Scarb;
use assert_fs::TempDir;
use std::fmt;
use url::Url;

#[cfg(feature = "scarb-config")]
use anyhow::Result;
#[cfg(feature = "scarb-config")]
use scarb::core::registry::index::{IndexRecord, IndexRecords};
#[cfg(feature = "scarb-config")]
use std::fs;
#[cfg(feature = "scarb-config")]
use std::io::{Read, Write};
#[cfg(feature = "scarb-config")]
use std::path::Path;

pub struct LocalRegistry {
    pub t: TempDir,
    pub url: String,
}

impl LocalRegistry {
    pub fn create() -> Self {
        let t = TempDir::new().unwrap();
        let url = Url::from_directory_path(&t).unwrap().to_string();
        Self { t, url }
    }

    pub fn publish(&mut self, f: impl FnOnce(&TempDir)) -> &mut Self {
        let t = TempDir::new().unwrap();
        f(&t);
        Scarb::quick_snapbox()
            .arg("publish")
            .arg("--no-verify")
            .arg("--index")
            .arg(&self.url)
            .current_dir(&t)
            .assert()
            .success();
        self
    }

    pub fn publish_verified(&mut self, f: impl FnOnce(&TempDir)) -> &mut Self {
        let t = TempDir::new().unwrap();
        f(&t);
        Scarb::quick_snapbox()
            .arg("publish")
            .arg("--index")
            .arg(&self.url)
            .current_dir(&t)
            .assert()
            .success();
        self
    }
}

impl fmt::Display for LocalRegistry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.url, f)
    }
}

/// Marks test package version yanked. Warning: does not modify cache.
#[cfg(feature = "scarb-config")]
pub fn yank(file_path: &Path, version: &str) -> Result<()> {
    with_package(file_path, version, |pkg| pkg.yanked = true)
}

/// Marks test package version as audited. Warning: does not modify cache.
#[cfg(feature = "scarb-config")]
pub fn audit(file_path: &Path, version: &str) -> Result<()> {
    with_package(file_path, version, |pkg| pkg.audited = true)
}

/// Unmarks test package version as audited. Warning: does not modify cache.
#[cfg(feature = "scarb-config")]
pub fn unaudit(file_path: &Path, version: &str) -> Result<()> {
    with_package(file_path, version, |pkg| pkg.audited = false)
}

/// Apply an arbitrary change to a test package version.
/// Warning: does not modify cache.
#[cfg(feature = "scarb-config")]
fn with_package<F>(file_path: &Path, version: &str, op: F) -> Result<()>
where
    F: FnOnce(&mut IndexRecord),
{
    let mut file = fs::File::open(file_path)?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;
    let mut packages: IndexRecords = serde_json::from_str(&contents)
        .map_err(|e| anyhow::anyhow!("Failed to deserialize JSON: {}", e))?;
    match packages
        .iter_mut()
        .find(|package| package.version.to_string() == version)
    {
        Some(pkg) => op(pkg),
        None => panic!("Package with version '{version}' not found."),
    }
    let modified_contents = serde_json::to_string_pretty(&packages)
        .map_err(|e| anyhow::anyhow!("Failed to serialize modified packages: {e}"))?;

    let mut file = fs::File::create(file_path)?;
    file.write_all(modified_contents.as_bytes())?;
    file.sync_all()?;
    Ok(())
}
