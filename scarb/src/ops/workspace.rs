use anyhow::Result;
use camino::Utf8Path;

use crate::core::config::Config;
use crate::core::package::Package;
use crate::core::source::SourceId;
use crate::core::workspace::Workspace;
use crate::ops;

#[tracing::instrument(level = "debug", skip(config))]
pub fn read_workspace<'c>(manifest_path: &Utf8Path, config: &'c Config) -> Result<Workspace<'c>> {
    let source_id = SourceId::for_path(manifest_path)?;
    read_workspace_impl(manifest_path, source_id, config)
}

#[tracing::instrument(level = "debug", skip(config))]
pub fn read_workspace_with_source_id<'c>(
    manifest_path: &Utf8Path,
    source_id: SourceId,
    config: &'c Config,
) -> Result<Workspace<'c>> {
    read_workspace_impl(manifest_path, source_id, config)
}

fn read_workspace_impl<'c>(
    manifest_path: &Utf8Path,
    source_id: SourceId,
    config: &'c Config,
) -> Result<Workspace<'c>> {
    let manifest = Box::new(ops::read_manifest(manifest_path, source_id)?);

    let package = Package::new(manifest.summary.package_id, manifest_path.into(), manifest);

    Workspace::from_single_package(package, config)
}
