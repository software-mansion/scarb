use std::collections::HashMap;

use anyhow::Result;

use crate::core::package::{Package, PackageId};
use crate::core::registry::Registry;
use crate::core::workspace::Workspace;
use crate::resolver;

pub struct WorkspaceResolution {
    pub packages: HashMap<PackageId, Package>,
}

/// Resolves workspace dependencies and downloads missing packages.
#[tracing::instrument(
    level = "debug",
    skip_all,
    fields(root = ws.root().display().to_string())
)]
pub fn resolve_workspace(ws: &Workspace<'_>) -> Result<WorkspaceResolution> {
    let mut registry = Registry::preloaded(ws.members(), ws.config());

    let members_summaries = ws
        .members()
        .map(|pkg| pkg.manifest.summary.clone())
        .collect::<Vec<_>>();

    let resolve = resolver::resolve(&members_summaries, &mut registry, ws.config())?;
    let packages = resolve.download_packages(&mut registry)?;
    Ok(WorkspaceResolution { packages })
}
