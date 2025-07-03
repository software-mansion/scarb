use crate::command::Scarb;
use anyhow::Result;
use assert_fs::TempDir;
use serde::{Deserialize, Serialize};
use std::io::{Read, Write};
use std::path::Path;
use std::{fmt, fs};
use url::Url;

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

#[derive(Serialize, Deserialize)]
struct Package {
    pub v: String,
    pub deps: Vec<String>,
    pub cksum: String,
    pub yanked: Option<bool>,
}

/// Marks test package yanked. Warning: does not modify cache.
pub fn yank(file_path: &Path, version: &str) -> Result<()> {
    let mut file = fs::File::open(file_path)?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;
    let mut packages: Vec<Package> = serde_json::from_str(&contents)
        .map_err(|e| anyhow::anyhow!("Failed to deserialize JSON: {}", e))?;
    match packages.iter_mut().find(|package| package.v == version) {
        Some(package) => package.yanked = Some(true),
        None => panic!("Package with version '{version}' not found."),
    }
    let modified_contents = serde_json::to_string_pretty(&packages)
        .map_err(|e| anyhow::anyhow!("Failed to serialize modified packages: {e}"))?;

    let mut file = fs::File::create(file_path)?;
    file.write_all(modified_contents.as_bytes())?;
    file.sync_all()?;
    Ok(())
}
