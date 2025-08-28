use std::collections::BTreeMap;

use crate::core::{
    MaybeWorkspaceTomlDependency, TomlCairoPluginTargetParams, TomlFeatureToEnable, TomlTarget,
};
use crate::{
    DEFAULT_LICENSE_FILE_NAME, DEFAULT_README_FILE_NAME,
    core::{
        DepKind, DependencyVersionReq, DetailedTomlDependency, ManifestDependency, MaybeWorkspace,
        Package, PackageName, TargetKind, TomlDependency, TomlManifest, TomlPackage,
    },
};
use anyhow::{Result, bail};
use camino::Utf8PathBuf;
use indoc::formatdoc;
use itertools::Itertools;
use smol_str::SmolStr;

pub fn prepare_manifest_for_publish(pkg: &Package) -> Result<TomlManifest> {
    let package = Some(generate_package(pkg));

    let dependencies = Some(generate_dependencies(
        // NOTE: We deliberately don't ask for `full_dependencies` here because
        // we don't want to emit requirements for built-in packages like `core`.
        &pkg.manifest.summary.dependencies,
        DepKind::Normal,
    )?);

    // NOTE: We used to emit an empty ` [dependencies]` table since packaging was introduced,
    //   so to avoid any potential breakages, we only nullify `[dev-dependencies]`.
    let dev_dependencies = nullify_table_if_empty(generate_dependencies(
        &pkg.manifest.summary.dependencies,
        DepKind::Target(TargetKind::TEST),
    )?);

    let tool = pkg.manifest.metadata.tool_metadata.clone().map(|m| {
        m.into_iter()
            .map(|(k, v)| (k, MaybeWorkspace::Defined(v)))
            .collect()
    });

    let cairo_plugin = generate_cairo_plugin(pkg);
    let features = nullify_table_if_empty(
        pkg.manifest
            .features
            .iter()
            // Sort for stability.
            .map(|(key, val)| {
                (
                    key.clone(),
                    val.iter()
                        .cloned()
                        .map(TomlFeatureToEnable::from)
                        .sorted()
                        .collect_vec(),
                )
            })
            .collect(),
    );

    Ok(TomlManifest {
        package,
        workspace: None,
        dependencies,
        dev_dependencies,
        lib: None,
        executable: None,
        cairo_plugin,
        test: None,
        target: None,
        cairo: None,
        profile: None,
        scripts: None,
        tool,
        features,
        patch: None,
        target_defaults: None,
    })
}

fn generate_package(pkg: &Package) -> Box<TomlPackage> {
    let summary = &pkg.manifest.summary;
    let metadata = &pkg.manifest.metadata;
    Box::new(TomlPackage {
        name: summary.package_id.name.clone(),
        version: MaybeWorkspace::Defined(summary.package_id.version.clone()),
        edition: Some(MaybeWorkspace::Defined(pkg.manifest.edition)),
        publish: (!pkg.manifest.publish).then_some(false),
        authors: metadata.authors.clone().map(MaybeWorkspace::Defined),
        urls: metadata.urls.clone(),
        description: metadata.description.clone().map(MaybeWorkspace::Defined),
        documentation: metadata.documentation.clone().map(MaybeWorkspace::Defined),
        homepage: metadata.homepage.clone().map(MaybeWorkspace::Defined),
        keywords: metadata.keywords.clone().map(MaybeWorkspace::Defined),
        license: metadata.license.clone().map(MaybeWorkspace::Defined),
        license_file: metadata
            .license_file
            .clone()
            .map(|_| MaybeWorkspace::Defined(Utf8PathBuf::from(DEFAULT_LICENSE_FILE_NAME))),
        readme: metadata
            .readme
            .clone()
            .map(|_| MaybeWorkspace::Defined(Utf8PathBuf::from(DEFAULT_README_FILE_NAME).into())),
        repository: metadata.repository.clone().map(MaybeWorkspace::Defined),
        include: metadata.include.as_ref().map(|x| {
            // Sort for stability.
            x.iter().sorted().cloned().collect_vec()
        }),
        no_core: summary.no_core.then_some(true),
        cairo_version: metadata.cairo_version.clone().map(MaybeWorkspace::Defined),
        experimental_features: pkg.manifest.experimental_features.clone(),
        re_export_cairo_plugins: Some(pkg.manifest.summary.re_export_cairo_plugins.clone()),
    })
}

fn generate_dependencies(
    deps: &[ManifestDependency],
    kind: DepKind,
) -> Result<BTreeMap<PackageName, MaybeWorkspaceTomlDependency>> {
    deps.iter()
        .filter(|dep| dep.kind == kind)
        .map(|dep| {
            let name = dep.name.clone();
            let toml_dep = generate_dependency(dep)?;
            Ok((name, MaybeWorkspace::Defined(toml_dep).into()))
        })
        .collect()
}

fn generate_dependency(dep: &ManifestDependency) -> Result<TomlDependency> {
    assert!(
        !dep.source_id.is_std(),
        "Built-in dependencies should not be included in published manifests."
    );

    let version = Some(match &dep.version_req {
        DependencyVersionReq::Req(req) => req.clone(),

        // Ignore what is in the lock file.
        DependencyVersionReq::Locked { req, .. } => req.clone(),

        // This case is triggered by dependencies like this:
        //
        // [dependencies]
        // foo = { path = "../foo" }
        DependencyVersionReq::Any => {
            bail!(formatdoc! {
                r#"
                    dependency `{name}` does not specify a version requirement
                    note: all dependencies must have a version specified when packaging
                    note: the `{kind}` specification will be removed from dependency declaration 
                "#,
                name = dep.name,
                kind = dep.source_id.kind.primary_field(),
            })
        }
    });

    let features = (!dep.features.is_empty()).then_some(
        dep.features
            .clone()
            .into_iter()
            .map(SmolStr::from)
            // Sort for stability.
            .sorted()
            .collect_vec(),
    );
    let default_features = (!dep.default_features).then_some(false);

    Ok(TomlDependency::Detailed(Box::new(DetailedTomlDependency {
        version,

        // Erase path information, effectively making the dependency default registry-based.
        path: None,

        // Same for Git specification.
        git: None,
        branch: None,
        tag: None,
        rev: None,

        // Unless it is the default registry, expand the registry specification to registry URL.
        //
        // NOTE: Default registry will reject packages with dependencies from other registries.
        registry: if dep.source_id.is_registry() && !dep.source_id.is_default_registry() {
            Some(dep.source_id.url.clone())
        } else {
            None
        },

        features,
        default_features,
    })))
}

fn generate_cairo_plugin(pkg: &Package) -> Option<TomlTarget<TomlCairoPluginTargetParams>> {
    let target = pkg.target(&TargetKind::CAIRO_PLUGIN)?;
    let params = target.props::<TomlCairoPluginTargetParams>().ok()?;

    Some(TomlTarget {
        name: Some(target.name.clone()),
        source_path: None,
        params: TomlCairoPluginTargetParams {
            builtin: params.builtin.and_then(|b| b.then_some(true)),
        },
    })
}

fn nullify_table_if_empty<K, V>(table: BTreeMap<K, V>) -> Option<BTreeMap<K, V>> {
    if table.is_empty() { None } else { Some(table) }
}
