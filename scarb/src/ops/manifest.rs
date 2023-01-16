use std::path::{Path, PathBuf};
use std::{env, fs};

use anyhow::{Context, Result};

use crate::core::manifest::{Manifest, TomlManifest};
use crate::core::source::SourceId;
use crate::MANIFEST_FILE_NAME;

#[tracing::instrument(level = "debug", skip_all)]
pub fn read_manifest(manifest_path: &Path, source_id: SourceId) -> anyhow::Result<Manifest> {
    let toml = TomlManifest::read_from_path(manifest_path)?;
    toml.to_manifest(manifest_path, source_id)
        .with_context(|| format!("failed to parse manifest at `{}`", &manifest_path.display()))
}

#[tracing::instrument(level = "debug")]
pub fn find_manifest_path(user_override: Option<&Path>) -> Result<PathBuf> {
    match user_override {
        Some(user_override) => {
            Ok(fs::canonicalize(user_override).unwrap_or_else(|_| user_override.into()))
        }
        None => try_find_manifest_of_pwd(),
    }
}

fn try_find_manifest_of_pwd() -> Result<PathBuf> {
    let pwd = env::current_dir()?;

    let mut root = Some(pwd.as_path());
    while let Some(path) = root {
        let manifest = path.join(MANIFEST_FILE_NAME);
        if manifest.is_file() {
            return Ok(manifest);
        }

        root = path.parent();
    }

    Ok(pwd.join(MANIFEST_FILE_NAME))
}
