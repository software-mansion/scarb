use std::{env, fs};

use crate::compiler::Profile;
use anyhow::{Context, Result};
use camino::{Utf8Path, Utf8PathBuf};

use crate::core::manifest::{Manifest, TomlManifest};
use crate::core::source::SourceId;
use crate::internal::fsx::{PathBufUtf8Ext, PathUtf8Ext};
use crate::MANIFEST_FILE_NAME;

#[tracing::instrument(level = "debug", skip_all)]
pub fn read_manifest(
    manifest_path: &Utf8Path,
    source_id: SourceId,
    profile: Profile,
) -> Result<Manifest> {
    let toml_manifest = TomlManifest::read_from_path(manifest_path)?;
    toml_manifest
        .to_manifest(manifest_path, source_id, profile, None)
        .with_context(|| format!("failed to parse manifest at `{manifest_path}`"))
}

#[tracing::instrument(level = "debug")]
pub fn find_manifest_path(user_override: Option<&Utf8Path>) -> Result<Utf8PathBuf> {
    match user_override {
        Some(user_override) => Ok(fs::canonicalize(user_override)
            .unwrap_or_else(|_| user_override.into())
            .try_into_utf8()?),
        None => try_find_manifest_of_pwd(),
    }
}

fn try_find_manifest_of_pwd() -> Result<Utf8PathBuf> {
    let pwd = env::current_dir()?.try_to_utf8()?;

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
