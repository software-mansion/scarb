use std::env;

use anyhow::Result;
use camino::{Utf8Path, Utf8PathBuf};

use crate::{MANIFEST_FILE_NAME, PathBufUtf8Ext, canonicalize, canonicalize_utf8};

/// Find a manifest file by walking up from the current directory.
///
/// When `user_override` is provided, it is canonicalized (best effort) and returned.
/// When no manifest is found, returns the path where it would be expected in the
/// current directory.
pub fn find_manifest_path(user_override: Option<&Utf8Path>) -> Result<Utf8PathBuf> {
    match user_override {
        Some(user_override) => Ok(canonicalize(user_override)
            .unwrap_or_else(|_| user_override.into())
            .try_into_utf8()?),
        None => {
            let pwd = canonicalize_utf8(env::current_dir()?)?;
            let accept_all = |_| Ok(true);
            let manifest_path = try_find_manifest_of_pwd(pwd.clone(), accept_all)?
                .unwrap_or_else(|| pwd.join(MANIFEST_FILE_NAME));
            Ok(manifest_path)
        }
    }
}

pub fn try_find_manifest_of_pwd(
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
