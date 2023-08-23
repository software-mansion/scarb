use std::collections::{BTreeMap, HashMap};

use anyhow::{bail, Result};
use itertools::Itertools;
use semver::{Version, VersionReq};
use smol_str::SmolStr;

use scarb_metadata as m;
use scarb_ui::args::PackagesSource;

use crate::compiler::CompilationUnit;
use crate::core::{
    DependencyVersionReq, ManifestDependency, Package, PackageId, SourceId, Target, Workspace,
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

    let (mut packages, mut compilation_units) = if !opts.no_deps {
        let resolve = ops::resolve_workspace(ws)?;
        let packages: Vec<m::PackageMetadata> = resolve
            .packages
            .values()
            .map(collect_package_metadata)
            .collect();

        let compilation_units: Vec<m::CompilationUnitMetadata> =
            ops::generate_compilation_units(&resolve, ws)?
                .iter()
                .map(collect_compilation_unit_metadata)
                .collect();

        (packages, compilation_units)
    } else {
        let packages = ws.members().map(|p| collect_package_metadata(&p)).collect();
        (packages, Vec::new())
    };

    packages.sort_by_key(|p| p.id.clone());
    compilation_units.sort_by_key(|c| c.package.clone());

    Ok(m::MetadataBuilder::default()
        .app_exe(ws.config().app_exe().ok().map(|p| p.to_path_buf()))
        .app_version_info(collect_app_version_metadata())
        .target_dir(Some(ws.target_dir().path_unchecked().to_path_buf()))
        .runtime_manifest(ws.runtime_manifest().clone())
        .workspace(collect_workspace_metadata(ws)?)
        .packages(packages)
        .compilation_units(compilation_units)
        .current_profile(ws.current_profile()?.to_string())
        .profiles(ws.profile_names()?)
        .build()
        .unwrap())
}

fn collect_workspace_metadata(ws: &Workspace<'_>) -> Result<m::WorkspaceMetadata> {
    let mut members: Vec<m::PackageId> = ws.members().map(|it| wrap_package_id(it.id)).collect();
    members.sort();

    Ok(m::WorkspaceMetadataBuilder::default()
        .manifest_path(ws.manifest_path())
        .root(ws.root())
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
        .root(package.root())
        .dependencies(dependencies)
        .targets(targets)
        .manifest_metadata(manifest_metadata)
        .build()
        .unwrap()
}

fn collect_dependency_metadata(dependency: &ManifestDependency) -> m::DependencyMetadata {
    let version_req = match &dependency.version_req {
        DependencyVersionReq::Any => VersionReq::STAR,
        DependencyVersionReq::Req(req) => req.clone(),
        DependencyVersionReq::Locked { req, .. } => req.clone(),
    };

    m::DependencyMetadataBuilder::default()
        .name(dependency.name.to_string())
        .version_req(version_req)
        .source(wrap_source_id(dependency.source_id))
        .build()
        .unwrap()
}

fn collect_target_metadata(target: &Target) -> m::TargetMetadata {
    m::TargetMetadataBuilder::default()
        .kind(target.kind.to_string())
        .name(target.name.to_string())
        .source_path(target.source_path.clone())
        .params(toml_to_json(&target.params))
        .build()
        .unwrap()
}

fn collect_compilation_unit_metadata(
    compilation_unit: &CompilationUnit,
) -> m::CompilationUnitMetadata {
    let components: Vec<m::CompilationUnitComponentMetadata> = compilation_unit
        .components
        .iter()
        .map(|c| {
            m::CompilationUnitComponentMetadataBuilder::default()
                .package(wrap_package_id(c.package.id))
                .name(c.cairo_package_name())
                .source_path(c.target.source_path.clone())
                .build()
                .unwrap()
        })
        .sorted_by_key(|c| c.package.clone())
        .collect();

    let cairo_plugins: Vec<m::CompilationUnitCairoPluginMetadata> = compilation_unit
        .cairo_plugins
        .iter()
        .map(|c| {
            m::CompilationUnitCairoPluginMetadataBuilder::default()
                .package(wrap_package_id(c.package.id))
                .build()
                .unwrap()
        })
        .sorted_by_key(|c| c.package.clone())
        .collect();

    let compiler_config = serde_json::to_value(&compilation_unit.compiler_config)
        .expect("Compiler config should always be JSON serializable.");

    let cfg = compilation_unit
        .cfg_set
        .iter()
        .map(|cfg| {
            serde_json::to_value(cfg)
                .and_then(serde_json::from_value::<m::Cfg>)
                .expect("Cairo's `Cfg` must serialize identically as Scarb Metadata's `Cfg`.")
        })
        .collect::<Vec<_>>();

    let components_legacy = components
        .iter()
        .map(|c| c.package.to_string())
        .collect::<Vec<_>>();

    m::CompilationUnitMetadataBuilder::default()
        .id(compilation_unit.id())
        .package(wrap_package_id(compilation_unit.main_package_id))
        .target(collect_target_metadata(compilation_unit.target()))
        .components(components)
        .cairo_plugins(cairo_plugins)
        .compiler_config(compiler_config)
        .cfg(cfg)
        .extra(HashMap::from([(
            "components".to_owned(),
            serde_json::Value::from(components_legacy),
        )]))
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
        .commit_date(ci.commit_date.map(Into::into))
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

fn btree_toml_to_json(map: &BTreeMap<SmolStr, toml::Value>) -> BTreeMap<String, serde_json::Value> {
    map.iter()
        .map(|(k, v)| (k.to_string(), toml_to_json(v)))
        .collect()
}

fn toml_to_json(value: &toml::Value) -> serde_json::Value {
    serde_json::to_value(value).expect("Conversion from TOML value to JSON value should not fail.")
}
