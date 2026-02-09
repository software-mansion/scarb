use anyhow::Result;
use camino::{Utf8Path, Utf8PathBuf};

use crate::core::manifest::TomlManifest;
use scarb_fs_utils as fsx;

#[tracing::instrument(level = "debug")]
pub fn find_manifest_path(user_override: Option<&Utf8Path>) -> Result<Utf8PathBuf> {
    fsx::find_manifest_path(user_override)
}

#[tracing::instrument(level = "debug")]
pub fn find_workspace_manifest_path(pkg_manifest_path: Utf8PathBuf) -> Result<Option<Utf8PathBuf>> {
    let is_workspace: fn(Utf8PathBuf) -> Result<bool> = |manifest_path| {
        TomlManifest::read_from_path(manifest_path.as_path()).map(|m| m.is_workspace())
    };
    fsx::try_find_manifest_of_pwd(pkg_manifest_path, is_workspace)
}
