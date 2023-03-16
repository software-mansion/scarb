use std::collections::BTreeMap;

use anyhow::{bail, Result};
use semver::Version;
use serde_json::json;

use scarb_metadata as m;

use crate::compiler::CompilationUnit;
use crate::core::{
    ExternalTargetKind, LibTargetKind, ManifestDependency, Package, PackageId, SourceId, Target,
    TargetKind, Workspace,
};
use crate::ops;
use crate::version::CommitInfo;

pub struct MetadataOptions {
    pub version: u64,
    pub no_deps: bool,
}

#[tracing::instrument(skip_all, level = "debug")]
pub fn collect_metadata(opts: &MetadataOptions, ws: &Workspace<'_>) -> Result<m::Metadata> {
    if opts.version != m::VersionPin.numeric() {
        bail!(
            "metadata version {} not supported, only {} is currently supported",
            opts.version,
            m::VersionPin
        );
    }

    let (mut packages, compilation_units) = if !opts.no_deps {
        let resolve = ops::resolve_workspace(ws)?;
        let packages: Vec<m::PackageMetadata> = resolve
            .packages
            .values()
            .map(collect_package_metadata)
            .collect();

        let mut compilation_units: Vec<m::CompilationUnitMetadata> =
            ops::generate_compilation_units(&resolve, ws)?
                .iter()
                .map(collect_compilation_unit_metadata)
                .collect();

        compilation_units.sort_by_key(|c| c.package.clone());

        (packages, compilation_units)
    } else {
        let packages = ws.members().map(|p| collect_package_metadata(&p)).collect();
        (packages, Vec::new())
    };

    packages.sort_by_key(|p| p.id.clone());

    Ok(m::MetadataBuilder::default()
        .app_exe(ws.config().app_exe().ok().map(|p| p.to_path_buf()))
        .app_version_info(collect_app_version_metadata())
        .target_dir(Some(ws.target_dir().path_unchecked().to_path_buf()))
        .workspace(collect_workspace_metadata(ws)?)
        .packages(packages)
        .compilation_units(compilation_units)
        .build()
        .unwrap())
}

fn collect_workspace_metadata(ws: &Workspace<'_>) -> Result<m::WorkspaceMetadata> {
    let mut members: Vec<m::PackageId> = ws.members().map(|it| wrap_package_id(it.id)).collect();
    members.sort();

    Ok(m::WorkspaceMetadataBuilder::default()
        .manifest_path(ws.manifest_path())
        .members(members)
        .build()
        .unwrap())
}

fn collect_package_metadata(package: &Package) -> m::PackageMetadata {
    let mut dependencies: Vec<m::DependencyMetadata> = package
        .manifest
        .summary
        .full_dependencies()
        .map(collect_dependency_metadata)
        .collect();
    dependencies.sort_by_key(|d| (d.name.clone(), d.source.clone()));

    let mut targets: Vec<m::TargetMetadata> = package
        .manifest
        .targets
        .iter()
        .map(collect_target_metadata)
        .collect();
    targets.sort_by_key(|t| (t.kind.clone(), t.name.clone()));

    let manifest_metadata = m::ManifestMetadataBuilder::default()
        .authors(package.manifest.metadata.authors.clone())
        .description(package.manifest.metadata.description.clone())
        .documentation(package.manifest.metadata.documentation.clone())
        .homepage(package.manifest.metadata.homepage.clone())
        .keywords(package.manifest.metadata.keywords.clone())
        .license(package.manifest.metadata.license.clone())
        .license_file(package.manifest.metadata.license_file.clone())
        .readme(package.manifest.metadata.readme.clone())
        .repository(package.manifest.metadata.repository.clone())
        .urls(package.manifest.metadata.urls.clone())
        .tool(
            package
                .manifest
                .metadata
                .tool_metadata
                .as_ref()
                .map(btree_toml_to_json),
        )
        .build()
        .unwrap();

    m::PackageMetadataBuilder::default()
        .id(wrap_package_id(package.id))
        .name(package.id.name.clone())
        .version(package.id.version.clone())
        .source(wrap_source_id(package.id.source_id))
        .manifest_path(package.manifest_path())
        .dependencies(dependencies)
        .targets(targets)
        .manifest_metadata(manifest_metadata)
        .build()
        .unwrap()
}

fn collect_dependency_metadata(dependency: &ManifestDependency) -> m::DependencyMetadata {
    m::DependencyMetadataBuilder::default()
        .name(dependency.name.to_string())
        .version_req(dependency.version_req.clone())
        .source(wrap_source_id(dependency.source_id))
        .build()
        .unwrap()
}

fn collect_target_metadata(target: &Target) -> m::TargetMetadata {
    let name = target.name.to_string();

    let (kind, params) = match &target.kind {
        TargetKind::Lib(LibTargetKind { sierra, casm }) => {
            let kind = "lib".to_string();
            let params = json!({
                "sierra": sierra,
                "casm": casm
            });
            (kind, params)
        }
        TargetKind::External(ExternalTargetKind { kind_name, params }) => {
            let kind = kind_name.to_string();
            let params = params
                .iter()
                .map(|(k, v)| (k.clone(), toml_to_json(v)))
                .collect();
            (kind, params)
        }
    };

    m::TargetMetadataBuilder::default()
        .name(name)
        .kind(kind)
        .params(params)
        .build()
        .unwrap()
}

fn collect_compilation_unit_metadata(
    compilation_unit: &CompilationUnit,
) -> m::CompilationUnitMetadata {
    let mut components: Vec<m::PackageId> = compilation_unit
        .components
        .iter()
        .map(|p| wrap_package_id(p.id))
        .collect();
    components.sort();

    let compiler_config = serde_json::to_value(&compilation_unit.compiler_config)
        .expect("Compiler config should always be JSON serializable.");

    m::CompilationUnitMetadataBuilder::default()
        .package(wrap_package_id(compilation_unit.package.id))
        .target(collect_target_metadata(&compilation_unit.target))
        .components(components)
        .compiler_config(compiler_config)
        .build()
        .unwrap()
}

fn collect_app_version_metadata() -> m::VersionInfo {
    let v = crate::version::get();

    let scarb_version: Version = v
        .version
        .parse()
        .expect("Scarb version should always be SemVer");

    let cairo_version: Version = v
        .cairo
        .version
        .parse()
        .expect("Cairo version should always be SemVer");

    let cairo = m::CairoVersionInfoBuilder::default()
        .version(cairo_version)
        .commit_info(v.cairo.commit_info.map(wrap_commit_info))
        .build()
        .unwrap();

    m::VersionInfoBuilder::default()
        .version(scarb_version)
        .commit_info(v.commit_info.map(wrap_commit_info))
        .cairo(cairo)
        .build()
        .unwrap()
}

fn wrap_commit_info(ci: CommitInfo) -> m::CommitInfo {
    m::CommitInfoBuilder::default()
        .short_commit_hash(ci.short_commit_hash)
        .commit_hash(ci.commit_hash)
        .commit_date(ci.commit_date)
        .build()
        .unwrap()
}

fn wrap_package_id(id: PackageId) -> m::PackageId {
    m::PackageId {
        repr: id.to_serialized_string(),
    }
}

fn wrap_source_id(id: SourceId) -> m::SourceId {
    m::SourceId {
        repr: id.to_pretty_url(),
    }
}

fn btree_toml_to_json(map: &BTreeMap<String, toml::Value>) -> BTreeMap<String, serde_json::Value> {
    map.iter()
        .map(|(k, v)| (k.clone(), toml_to_json(v)))
        .collect()
}

fn toml_to_json(value: &toml::Value) -> serde_json::Value {
    serde_json::to_value(value).expect("Conversion from TOML value to JSON value should not fail.")
}
