// NOTE: All collections must have stable sorting in order to provide reproducible outputs!

use std::collections::BTreeMap;
use std::path::PathBuf;

use anyhow::{bail, Result};
use camino::Utf8PathBuf;
use semver::{Version, VersionReq};
use serde::{Deserialize, Serialize};
use toml::Value;

pub use metadata_version::*;

use crate::compiler::{CompilationUnit, Profile};
use crate::core::manifest::{
    ExternalTargetKind, LibTargetKind, ManifestCompilerConfig, ManifestMetadata, Target, TargetKind,
};
use crate::core::{ManifestDependency, Package, PackageId, SourceId, Workspace};
use crate::ops;
use crate::version::VersionInfo;

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
    pub app_version_info: VersionInfo,
    pub target_dir: Option<Utf8PathBuf>,
    pub workspace: WorkspaceMetadata,
    pub packages: Vec<PackageMetadata>,
    pub compilation_units: Vec<CompilationUnitMetadata>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct WorkspaceMetadata {
    pub root: Utf8PathBuf,
    pub members: Vec<PackageId>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PackageMetadata {
    pub id: PackageId,
    pub name: String,
    pub version: Version,
    pub source: SourceId,
    pub root: Utf8PathBuf,
    pub manifest_path: Utf8PathBuf,
    pub dependencies: Vec<DependencyMetadata>,
    pub targets: Vec<TargetMetadata>,

    #[serde(flatten)]
    pub manifest_metadata: ManifestMetadata,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct DependencyMetadata {
    pub name: String,
    pub version_req: VersionReq,
    pub source: SourceId,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct TargetMetadata {
    pub kind: String,
    pub name: String,
    pub params: BTreeMap<String, Value>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CompilationUnitMetadata {
    pub package: PackageId,
    pub target: TargetMetadata,
    pub components: Vec<PackageId>,
    pub profile: Profile,
    pub compiler_config: ManifestCompilerConfig,
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
        let (mut packages, compilation_units) = if !opts.no_deps {
            let resolve = ops::resolve_workspace(ws)?;
            let packages: Vec<PackageMetadata> = resolve
                .packages
                .values()
                .map(PackageMetadata::new)
                .collect();

            let mut compilation_units: Vec<CompilationUnitMetadata> =
                ops::generate_compilation_units(&resolve, ws)?
                    .iter()
                    .map(CompilationUnitMetadata::new)
                    .collect();

            compilation_units.sort_by_key(|c| c.package);

            (packages, compilation_units)
        } else {
            let packages = ws.members().map(|p| PackageMetadata::new(&p)).collect();
            (packages, Vec::new())
        };

        packages.sort_by_key(|p| p.id);

        Ok(Self {
            version: MetadataVersionPin::<1>,
            app_exe: ws.config().app_exe().ok().map(Into::into),
            app_version_info: crate::version::get(),
            target_dir: Some(ws.target_dir().path_unchecked().to_path_buf()),
            workspace: WorkspaceMetadata::new(ws)?,
            packages,
            compilation_units,
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
            .full_dependencies()
            .map(DependencyMetadata::new)
            .collect();
        dependencies.sort_by_key(|d| (d.name.clone(), d.source));

        let mut targets: Vec<TargetMetadata> = package
            .manifest
            .targets
            .iter()
            .map(TargetMetadata::new)
            .collect();
        targets.sort_by_key(|t| (t.kind.clone(), t.name.clone()));

        Self {
            id: package.id,
            name: package.id.name.to_string(),
            version: package.id.version.clone(),
            source: package.id.source_id,
            root: package.root().to_path_buf(),
            manifest_path: package.manifest_path().to_path_buf(),
            dependencies,
            targets,
            manifest_metadata: package.manifest.metadata.clone(),
        }
    }
}

impl DependencyMetadata {
    pub fn new(dependency: &ManifestDependency) -> Self {
        Self {
            name: dependency.name.to_string(),
            version_req: dependency.version_req.clone(),
            source: dependency.source_id,
        }
    }
}

impl TargetMetadata {
    pub fn new(target: &Target) -> Self {
        let name = target.name.to_string();

        let (kind, params) = match &target.kind {
            TargetKind::Lib(LibTargetKind { sierra, casm }) => {
                let kind = "lib".to_string();
                let params = BTreeMap::from([
                    ("sierra".to_string(), Value::from(*sierra)),
                    ("casm".to_string(), Value::from(*casm)),
                ]);
                (kind, params)
            }
            TargetKind::External(ExternalTargetKind { kind_name, params }) => {
                let kind = kind_name.to_string();
                let params = params
                    .iter()
                    .map(|(k, v)| (k.to_string(), v.clone()))
                    .collect();
                (kind, params)
            }
        };

        TargetMetadata { kind, name, params }
    }
}

impl CompilationUnitMetadata {
    pub fn new(compilation_unit: &CompilationUnit) -> Self {
        let mut components: Vec<PackageId> =
            compilation_unit.components.iter().map(|p| p.id).collect();
        components.sort();
        Self {
            package: compilation_unit.package.id,
            target: TargetMetadata::new(&compilation_unit.target),
            components,
            profile: compilation_unit.profile.clone(),
            compiler_config: compilation_unit.compiler_config.clone(),
        }
    }
}
