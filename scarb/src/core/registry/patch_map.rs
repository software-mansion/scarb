use crate::core::{ManifestDependency, PackageName, SourceId};
use crate::sources::canonical_url::CanonicalUrl;
use scarb_ui::Ui;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct PatchMap {
    map: HashMap<CanonicalUrl, HashMap<PackageName, ManifestDependency>>,
    unused: RefCell<HashSet<(CanonicalUrl, PackageName, SourceId)>>,
}

impl PatchMap {
    pub fn new() -> Self {
        Default::default()
    }

    /// Lookup the `dependency` in this patch map and return patched dependency if found,
    /// or return `dependency` back otherwise.
    pub fn lookup(&self, dependency: &ManifestDependency) -> ManifestDependency {
        let source_pattern = &dependency.source_id.canonical_url;
        let patch_dep = self
            .map
            .get(source_pattern)
            .and_then(|patches| patches.get(&dependency.name));

        let result = if let Some(patch_dep) = patch_dep {
            ManifestDependency::builder()
                .name(patch_dep.name.clone())
                .version_req(patch_dep.version_req.clone())
                .source_id(patch_dep.source_id)
                .kind(dependency.kind.clone())
                .features(patch_dep.features.clone())
                .default_features(patch_dep.default_features)
                .build()
        } else {
            dependency.clone()
        };

        self.unused.borrow_mut().remove(&(
            source_pattern.clone(),
            result.name.clone(),
            result.source_id,
        ));
        result
    }

    pub fn insert(
        &mut self,
        source_pattern: CanonicalUrl,
        dependencies: impl IntoIterator<Item = ManifestDependency>,
    ) {
        for dependency in dependencies.into_iter() {
            self.unused.borrow_mut().insert((
                source_pattern.clone(),
                dependency.name.clone(),
                dependency.source_id,
            ));
            self.map
                .entry(source_pattern.clone())
                .or_default()
                .insert(dependency.name.clone(), dependency);
        }
    }

    pub fn warn_unused(&self, ui: Ui) {
        for (source_url, package_name, source_id) in self.unused.borrow().iter() {
            if !source_id.is_std() {
                ui.warn(format!(
                    "patch `{package_name}` (`{source_id}`) for source `{source_url}` has not been used",
                ));
            }
        }
    }
}
