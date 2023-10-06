use std::collections::BTreeMap;

use anyhow::{bail, Result};
use camino::{Utf8Path, Utf8PathBuf};
use indoc::formatdoc;

use crate::core::{
    DepKind, DependencyVersionReq, DetailedTomlDependency, ManifestDependency, MaybeWorkspace,
    Package, PackageName, TomlDependency, TomlManifest, TomlPackage, TomlWorkspaceDependency,
    TomlWorkspaceField,
};

pub fn prepare_manifest_for_publish(pkg: &Package) -> Result<TomlManifest> {
    let package = Some(generate_package(pkg));

    let dependencies = Some(generate_dependencies(
        // NOTE: We deliberately do not ask for `full_dependencies` here, because
        // we do not want to emit requirements for built-in packages like `core`.
        &pkg.manifest.summary.dependencies,
        DepKind::Normal,
    )?);

    let tool = pkg.manifest.metadata.tool_metadata.clone().map(|m| {
        m.into_iter()
            .map(|(k, v)| (k, MaybeWorkspace::Defined(v)))
            .collect()
    });

    Ok(TomlManifest {
        package,
        workspace: None,
        dependencies,
        lib: None,
        // TODO(mkaput): Allow packaging Cairo plugins.
        cairo_plugin: None,
        test: None,
        target: None,
        cairo: None,
        profile: None,
        scripts: None,
        tool,
    })
}

fn generate_package(pkg: &Package) -> Box<TomlPackage> {
    let summary = &pkg.manifest.summary;
    let metadata = &pkg.manifest.metadata;
    Box::new(TomlPackage {
        name: summary.package_id.name.clone(),
        version: MaybeWorkspace::Defined(summary.package_id.version.clone()),
        authors: metadata.authors.clone().map(MaybeWorkspace::Defined),
        urls: metadata.urls.clone(),
        description: metadata.description.clone().map(MaybeWorkspace::Defined),
        documentation: metadata.documentation.clone().map(MaybeWorkspace::Defined),
        homepage: metadata.homepage.clone().map(MaybeWorkspace::Defined),
        keywords: metadata.keywords.clone().map(MaybeWorkspace::Defined),
        license: metadata.license.clone().map(MaybeWorkspace::Defined),
        // TODO(mkaput): Normalize this the same way as readme is.
        license_file: metadata.license_file.clone().map(MaybeWorkspace::Defined),
        readme: metadata
            .readme
            .as_ref()
            .map(|p| map_metadata_file_path(p, pkg)),
        repository: metadata.repository.clone().map(MaybeWorkspace::Defined),
        no_core: summary.no_core.then_some(true),
        cairo_version: metadata.cairo_version.clone().map(MaybeWorkspace::Defined),
    })
}

fn generate_dependencies(
    deps: &[ManifestDependency],
    kind: DepKind,
) -> Result<BTreeMap<PackageName, MaybeWorkspace<TomlDependency, TomlWorkspaceDependency>>> {
    deps.iter()
        .filter(|dep| dep.kind == kind)
        .map(|dep| {
            let name = dep.name.clone();
            let toml_dep = generate_dependency(dep)?;
            Ok((name, MaybeWorkspace::Defined(toml_dep)))
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

    Ok(TomlDependency::Detailed(DetailedTomlDependency {
        version,
        path: None,
        git: None,
        branch: None,
        tag: None,
        rev: None,
    }))
}

fn map_metadata_file_path<T>(
    path: &Utf8Path,
    pkg: &Package,
) -> MaybeWorkspace<T, TomlWorkspaceField>
where
    T: From<Utf8PathBuf>,
{
    assert!(
        path.is_absolute(),
        "Manifest parser is expected to canonicalize paths for README/LICENSE files."
    );

    let path = if let Ok(relative_path) = path.strip_prefix(pkg.root()) {
        relative_path.to_owned()
    } else {
        // This path points outside the package root. `scarb package` will copy it
        // into the root, so we have to adjust the path to this location.
        path.file_name()
            .expect("README/LICENSE path must have a file name.")
            .into()
    };

    MaybeWorkspace::Defined(T::from(path))
}
