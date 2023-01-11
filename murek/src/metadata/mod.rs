// NOTE: All collections must have stable sorting in order to provide reproducible outputs!

use std::path::PathBuf;

use anyhow::{bail, Result};
use semver::{Version, VersionReq};
use serde::{Deserialize, Serialize};
use smol_str::SmolStr;

pub use metadata_version::*;

use crate::core::manifest::ManifestMetadata;
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
    pub packages: Vec<PackageMetadata>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct WorkspaceMetadata {
    pub root: PathBuf,
    pub members: Vec<PackageId>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PackageMetadata {
    pub id: PackageId,
    pub name: SmolStr,
    pub version: Version,
    pub source: SourceId,
    pub root: PathBuf,
    pub manifest_path: PathBuf,
    pub dependencies: Vec<DependencyMetadata>,

    #[serde(flatten)]
    pub manifest_metadata: ManifestMetadata,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct DependencyMetadata {
    pub name: SmolStr,
    pub version_req: VersionReq,
    pub source: SourceId,
    // TODO(mkaput): Perhaps point to resolved package id here?
    //   This will make it easier for consumers to navigate the output.
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
        let mut packages: Vec<PackageMetadata> = if !opts.no_deps {
            let resolve = resolve_workspace(ws)?;
            resolve
                .packages
                .values()
                .map(PackageMetadata::new)
                .collect()
        } else {
            ws.members().map(|p| PackageMetadata::new(&p)).collect()
        };

        packages.sort_by_key(|p| p.id);

        Ok(Self {
            version: MetadataVersionPin::<1>,
            app_exe: ws.config().app_exe().ok().map(Into::into),
            target_dir: Some(ws.config().target_dir.as_unchecked().to_path_buf()),
            workspace: WorkspaceMetadata::new(ws)?,
            packages,
        })
    }
}

impl WorkspaceMetadata {
    pub fn new(ws: &Workspace<'_>) -> Result<Self> {
        let mut members: Vec<PackageId> = ws.members().map(|it| it.id).collect();
        members.sort();

        Ok(Self {
            root: ws.root().into(),
            members,
        })
    }
}

impl PackageMetadata {
    pub fn new(package: &Package) -> Self {
        let mut dependencies: Vec<DependencyMetadata> = package
            .manifest
            .summary
            .dependencies
            .iter()
            .map(DependencyMetadata::new)
            .collect();
        dependencies.sort_by_key(|d| (d.name.clone(), d.source));

        Self {
            id: package.id,
            name: package.id.name.clone(),
            version: package.id.version.clone(),
            source: package.id.source_id,
            root: package.root().to_path_buf(),
            manifest_path: package.manifest_path().to_path_buf(),
            dependencies,
            manifest_metadata: package.manifest.metadata.clone(),
        }
    }
}

impl DependencyMetadata {
    pub fn new(dependency: &ManifestDependency) -> Self {
        Self {
            name: dependency.name.clone(),
            version_req: dependency.version_req.clone(),
            source: dependency.source_id,
        }
    }
}
