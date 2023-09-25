use std::collections::BTreeMap;

use anyhow::Result;
use itertools::Itertools;

use crate::core::{
    MaybeWorkspace, Package, PathOrBool, TomlExternalTargetParams, TomlManifest, TomlPackage,
    TomlTarget, TomlTargetKind, Workspace,
};

pub fn prepare_manifest_for_publish(pkg: &Package, _ws: &Workspace<'_>) -> Result<TomlManifest> {
    let manifest = &pkg.manifest;
    let summary = &manifest.summary;
    let metadata = &manifest.metadata;

    let package = Some(Box::new(TomlPackage {
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
        readme: match &metadata.readme {
            None => None,
            Some(path) => {
                // NOTE: We assume here that the packaging logic will put the readme next
                // to the generated Scarb.toml.
                let file_name = path
                    .file_name()
                    .expect("Readme path must have a file name.")
                    .into();
                Some(MaybeWorkspace::Defined(PathOrBool::Path(file_name)))
            }
        },
        repository: metadata.repository.clone().map(MaybeWorkspace::Defined),
        no_core: summary.no_core.then_some(true),
        cairo_version: metadata.cairo_version.clone().map(MaybeWorkspace::Defined),
    }));

    let dependencies = todo!();

    let target = Some(BTreeMap::from_iter(
        manifest
            .targets
            .iter()
            .map(|target| {
                let kind = TomlTargetKind::try_new(target.kind.clone()).expect(
                    "Already validated target kind should be able to be written back to TOML.",
                );

                let name = Some(target.name.clone());
                let source_path = Some(
                    target
                        .source_path
                        .strip_prefix(pkg.root())
                        .expect("Source paths should always be within package root directory.")
                        .to_path_buf(),
                );

                let params = target
                    .params
                    .clone()
                    .try_into::<TomlExternalTargetParams>()
                    .expect(
                        "Internally stored target params should always be \
                        a string-keyed map-like structure.",
                    );

                let toml_target = TomlTarget {
                    name,
                    source_path,
                    params,
                };
                (kind, toml_target)
            })
            .into_group_map(),
    ));

    let tool = metadata.tool_metadata.clone().map(|m| {
        m.into_iter()
            .map(|(k, v)| (k, MaybeWorkspace::Defined(v)))
            .collect()
    });

    Ok(TomlManifest {
        package,
        workspace: None,
        dependencies,
        lib: None,
        cairo_plugin: None,
        test: None,
        target,
        cairo: None,
        profile: None,
        scripts: None,
        tool,
    })
}
