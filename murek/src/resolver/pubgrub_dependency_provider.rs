use std::borrow::Borrow;
use std::cell::RefCell;
use std::collections::HashMap;
use std::error::Error;
use std::result::Result as StdResult;

use pubgrub::range::Range;
use pubgrub::solver::{choose_package_with_fewest_versions, Dependencies, DependencyProvider};
use tracing::trace;

use crate::core::registry::cache::RegistryCache;
use crate::core::ManifestDependency;
use crate::resolver::pubgrub_types::{
    package_id_from_pubgrub, pubgrub_range_from_version_req_and_source_id,
    version_req_and_source_id_from_pubgrub_range, PubGrubPackage, PubGrubVersion,
};

pub struct RegistryDependencyProvider<'r, 'c> {
    registry_cell: RefCell<&'r mut RegistryCache<'c>>,
}

impl<'r, 'c> RegistryDependencyProvider<'r, 'c> {
    pub fn new(registry: &'r mut RegistryCache<'c>) -> Self {
        Self {
            registry_cell: RefCell::new(registry),
        }
    }
}

impl<'r, 'c> DependencyProvider<PubGrubPackage, PubGrubVersion>
    for RegistryDependencyProvider<'r, 'c>
{
    #[tracing::instrument(level = "trace", skip_all)]
    fn choose_package_version<T, U>(
        &self,
        potential_packages: impl Iterator<Item = (T, U)>,
    ) -> StdResult<(T, Option<PubGrubVersion>), Box<dyn Error>>
    where
        T: Borrow<PubGrubPackage>,
        U: Borrow<Range<PubGrubVersion>>,
    {
        let potential_packages: Vec<_> = potential_packages.collect();
        trace!(num_candidates = potential_packages.len());

        let manifest_dependencies: Vec<_> = potential_packages
            .iter()
            .map(|(p, v)| {
                let name = p.borrow().name.clone();
                let (version_req, source_id) =
                    version_req_and_source_id_from_pubgrub_range(v.borrow());
                ManifestDependency {
                    name,
                    version_req,
                    source_id,
                }
            })
            .collect();

        let manifest_dependencies_refs: Vec<_> = manifest_dependencies.iter().collect();

        let mut registry = self.registry_cell.borrow_mut();
        let summaries = registry.query_many(&manifest_dependencies_refs)?;
        assert_eq!(potential_packages.len(), summaries.len());

        let versions: HashMap<PubGrubPackage, Vec<PubGrubVersion>> = potential_packages
            .iter()
            .zip(summaries.into_iter())
            .map(|((pubgrub_package, _), package_summaries)| {
                let mut versions: Vec<PubGrubVersion> = package_summaries
                    .into_iter()
                    .map(|summary| summary.package_id.into())
                    .collect();
                versions.sort_unstable_by(|a, b| b.cmp(a));
                (pubgrub_package.borrow().clone(), versions)
            })
            .collect();

        let candidate = choose_package_with_fewest_versions(
            |pkg| versions.get(pkg).unwrap().iter().cloned(),
            potential_packages.into_iter(),
        );

        trace!(
            candidate = %candidate.0.borrow(),
            version = candidate.1.as_ref().map(ToString::to_string)
        );

        Ok(candidate)
    }

    #[tracing::instrument(level = "trace", skip(self))]
    fn get_dependencies(
        &self,
        package: &PubGrubPackage,
        version: &PubGrubVersion,
    ) -> StdResult<Dependencies<PubGrubPackage, PubGrubVersion>, Box<dyn Error>> {
        let package_id = package_id_from_pubgrub(package, version);

        let mut registry = self.registry_cell.borrow_mut();
        let package = registry.download(package_id)?;

        let constraints = package
            .manifest
            .summary
            .dependencies
            .iter()
            .map(|dep| {
                (
                    dep.name.clone().into(),
                    pubgrub_range_from_version_req_and_source_id(
                        dep.version_req.clone(),
                        dep.source_id,
                    ),
                )
            })
            .collect();
        Ok(Dependencies::Known(constraints))
    }
}
