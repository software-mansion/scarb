use std::path::PathBuf;

use anyhow::{bail, Result};
use semver::{Version, VersionReq};
use serde::{Deserialize, Serialize};
use smol_str::SmolStr;

pub use metadata_version::*;

use crate::core::{ManifestDependency, Package, PackageId, SourceId, Workspace};
use crate::ops::resolve_workspace;

mod metadata_version;

pub struct MetadataOptions {
    pub version: MetadataVersion,
    pub no_deps: bool,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(untagged)]
pub enum Metadata {
    V1(ProjectMetadata),
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ProjectMetadata {
    pub version: MetadataVersionPin<1>,
    pub app_exe: Option<PathBuf>,
    pub target_dir: Option<PathBuf>,
    pub workspace: WorkspaceMetadata,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct WorkspaceMetadata {
    pub workspace_root: PathBuf,
    pub members: Vec<PackageId>,
    pub packages: Vec<PackageMetadata>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PackageMetadata {
    pub name: SmolStr,
    pub version: Version,
    pub id: PackageId,
    pub source: SourceId,
    pub root: PathBuf,
    pub manifest_path: PathBuf,
    pub dependencies: Vec<DependencyMetadata>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct DependencyMetadata {
    pub name: SmolStr,
    pub version: VersionReq,
    pub source_id: SourceId,
}

impl Metadata {
    pub fn collect(ws: &Workspace<'_>, opts: &MetadataOptions) -> Result<Self> {
        if opts.version != MetadataVersion::V1 {
            bail!(
                "metadata version {} not supported, only {} is currently supported",
                opts.version,
                MetadataVersion::V1
            );
        }

        ProjectMetadata::collect(ws, opts).map(Self::V1)
    }
}

impl ProjectMetadata {
    pub fn collect(ws: &Workspace<'_>, opts: &MetadataOptions) -> Result<Self> {
        Ok(Self {
            version: MetadataVersionPin::<1>,
            app_exe: ws.config().app_exe().ok().map(Into::into),
            target_dir: ws.config().target_dir().ok().map(|it| it.path.clone()),
            workspace: WorkspaceMetadata::collect(ws, opts)?,
        })
    }
}

impl WorkspaceMetadata {
    pub fn collect(ws: &Workspace<'_>, opts: &MetadataOptions) -> Result<Self> {
        let packages = if opts.no_deps {
            let resolve = resolve_workspace(ws)?;
            resolve
                .packages
                .values()
                .cloned()
                .map(PackageMetadata::new)
                .collect()
        } else {
            ws.members().map(PackageMetadata::new).collect()
        };

        Ok(Self {
            workspace_root: ws.root().into(),
            members: ws.members().map(|it| it.id).collect(),
            packages,
        })
    }
}

impl PackageMetadata {
    pub fn new(package: Package) -> Self {
        Self {
            name: package.id.name.clone(),
            version: package.id.version.clone(),
            id: package.id,
            source: package.id.source_id,
            root: package.root().to_path_buf(),
            manifest_path: package.manifest_path().to_path_buf(),
            // TODO(mkaput): Implement this.
            dependencies: Vec::new(),
        }
    }
}

impl DependencyMetadata {
    pub fn new(dependency: ManifestDependency) -> Self {
        Self {
            name: dependency.name,
            version: dependency.version,
            source_id: dependency.source_id,
        }
    }
}
