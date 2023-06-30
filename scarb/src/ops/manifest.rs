use std::{env, fs};

use anyhow::Result;
use camino::{Utf8Path, Utf8PathBuf};

use crate::core::manifest::TomlManifest;
use crate::internal::fsx::{PathBufUtf8Ext, PathUtf8Ext};
use crate::MANIFEST_FILE_NAME;

#[tracing::instrument(level = "debug")]
pub fn find_manifest_path(user_override: Option<&Utf8Path>) -> Result<Utf8PathBuf> {
    match user_override {
        Some(user_override) => Ok(fs::canonicalize(user_override)
            .unwrap_or_else(|_| user_override.into())
            .try_into_utf8()?),
        None => {
            let pwd = env::current_dir()?.try_to_utf8()?;
            let accept_all = |_| Ok(true);
            let manifest_path = try_find_manifest_of_pwd(pwd.clone(), accept_all)?
                .unwrap_or_else(|| pwd.join(MANIFEST_FILE_NAME));
            Ok(manifest_path)
        }
    }
}

#[tracing::instrument(level = "debug")]
pub fn find_workspace_manifest_path(pkg_manifest_path: Utf8PathBuf) -> Result<Option<Utf8PathBuf>> {
    let is_workspace: fn(Utf8PathBuf) -> Result<bool> = |manifest_path| {
        TomlManifest::read_from_path(manifest_path.as_path()).map(|m| m.is_workspace())
    };
    try_find_manifest_of_pwd(pkg_manifest_path, is_workspace)
}

fn try_find_manifest_of_pwd(
    pwd: Utf8PathBuf,
    accept: impl Fn(Utf8PathBuf) -> Result<bool>,
) -> Result<Option<Utf8PathBuf>> {
    let mut root = Some(pwd.as_path());
    while let Some(path) = root {
        let manifest = path.join(MANIFEST_FILE_NAME);
        if manifest.is_file() && accept(manifest.clone())? {
            return Ok(Some(manifest));
        }
        root = path.parent();
    }
    Ok(None)
}
