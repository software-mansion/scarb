use std::collections::HashMap;

use crate::core::{ManifestDependency, PackageName};
use crate::sources::canonical_url::CanonicalUrl;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct PatchMap(HashMap<CanonicalUrl, HashMap<PackageName, ManifestDependency>>);

impl PatchMap {
    pub fn new() -> Self {
        Default::default()
    }

    /// Lookup the `dependency` in this patch map and return patched dependency if found,
    /// or return `dependency` back otherwise.
    pub fn lookup<'a>(&'a self, dependency: &'a ManifestDependency) -> &'a ManifestDependency {
        self.0
            .get(&dependency.source_id.canonical_url)
            .and_then(|patches| patches.get(&dependency.name))
            .unwrap_or(dependency)
    }

    pub fn insert(
        &mut self,
        source_pattern: CanonicalUrl,
        dependencies: impl IntoIterator<Item = ManifestDependency>,
    ) {
        self.0.entry(source_pattern).or_default().extend(
            dependencies
                .into_iter()
                .map(|dependency| (dependency.name.clone(), dependency)),
        );
    }
}
