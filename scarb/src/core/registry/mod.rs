use anyhow::Result;
use async_trait::async_trait;

use crate::core::{ManifestDependency, Package, PackageId, Summary};

pub mod cache;
pub mod client;
pub mod index;
pub mod package_source_store;
pub mod patch_map;
pub mod patcher;
pub mod source_map;

pub const DEFAULT_REGISTRY_INDEX: &str = "https://scarbs.xyz/";

#[async_trait(?Send)]
pub trait Registry {
    /// Attempt to find the packages that match a dependency request.
    async fn query(&self, dependency: &ManifestDependency) -> Result<Vec<Summary>>;

    /// Fetch full package by its ID.
    async fn download(&self, package_id: PackageId) -> Result<Package>;
}

#[cfg(test)]
pub(crate) mod mock {
    use std::collections::{HashMap, HashSet};
    use std::sync::RwLock;

    use anyhow::{anyhow, bail, Result};
    use async_trait::async_trait;
    use camino::Utf8PathBuf;
    use itertools::Itertools;

    use crate::core::package::PackageName;
    use crate::core::registry::Registry;
    use crate::core::{ManifestBuilder, ManifestDependency, Package, PackageId, SourceId, Summary};

    #[derive(Debug, Default)]
    pub struct MockRegistry {
        index: HashMap<(PackageName, SourceId), HashSet<PackageId>>,
        dependencies: HashMap<PackageId, Vec<ManifestDependency>>,
        packages: RwLock<HashMap<PackageId, Package>>,
    }

    impl MockRegistry {
        pub fn new() -> Self {
            let mut reg = Self::default();
            reg.put(
                PackageId::new(
                    PackageName::CORE,
                    crate::version::get().cairo.version.parse().unwrap(),
                    SourceId::default(),
                ),
                Vec::new(),
            );
            reg.put(
                PackageId::new(
                    PackageName::TEST_PLUGIN,
                    crate::version::get().cairo.version.parse().unwrap(),
                    SourceId::default(),
                ),
                Vec::new(),
            );
            reg
        }

        pub fn put(&mut self, package_id: PackageId, mut dependencies: Vec<ManifestDependency>) {
            assert!(
                !self.has_package(package_id),
                "Package {package_id} is already in registry"
            );

            self.index
                .entry((package_id.name.clone(), package_id.source_id))
                .or_default()
                .insert(package_id);

            self.dependencies
                .entry(package_id)
                .or_default()
                .append(&mut dependencies);
        }

        pub fn has_package(&self, package_id: PackageId) -> bool {
            self.dependencies.contains_key(&package_id)
        }

        pub fn get_package(&self, package_id: PackageId) -> Result<Package> {
            if !self.has_package(package_id) {
                bail!("MockRegistry/get_package: unknown package {package_id}");
            }

            let packages = self.packages.read().unwrap();
            if packages.contains_key(&package_id) {
                Ok(packages[&package_id].clone())
            } else {
                drop(packages);

                let package =
                    Self::build_package(package_id, self.dependencies[&package_id].clone());

                let mut packages = self.packages.write().unwrap();
                packages.insert(package_id, package.clone());

                Ok(package)
            }
        }

        fn build_package(package_id: PackageId, dependencies: Vec<ManifestDependency>) -> Package {
            let summary = Summary::builder()
                .package_id(package_id)
                .dependencies(dependencies)
                .no_core(package_id.name == PackageName::CORE)
                .build();

            let manifest = Box::new(
                ManifestBuilder::default()
                    .summary(summary)
                    .targets(vec![])
                    .build()
                    .unwrap(),
            );

            Package::new(package_id, Utf8PathBuf::new(), manifest)
        }
    }

    #[async_trait(?Send)]
    impl Registry for MockRegistry {
        async fn query(&self, dependency: &ManifestDependency) -> Result<Vec<Summary>> {
            Ok(self
                .index
                .get(&(dependency.name.clone(), dependency.source_id))
                .ok_or_else(|| anyhow!("MockRegistry/query: cannot find {dependency}"))?
                .iter()
                .copied()
                .filter(|id| dependency.version_req.matches(&id.version))
                .sorted_unstable_by(|a, b| b.version.cmp(&a.version))
                .map(|id| self.get_package(id).unwrap().manifest.summary.clone())
                .collect())
        }

        async fn download(&self, package_id: PackageId) -> Result<Package> {
            self.get_package(package_id)
        }
    }

    macro_rules! registry {
        [$($x:tt),* $(,)?] => {{
            #[allow(unused_imports)]
            use $crate::core::registry::mock;
            #[allow(unused_mut)]
            let mut registry = mock::MockRegistry::new();
            $({
                let (package_id, dependencies) = mock::registry_entry!($x);
                registry.put(package_id, dependencies);
            })*
            registry
        }};
    }

    pub(crate) use registry;

    macro_rules! registry_entry {
        (($p:literal, [ $($d:tt),* $(,)? ] $(,)?)) => {{
            #[allow(unused_imports)]
            use $crate::core::registry::mock;
            let package_id = $crate::core::PackageId::from_display_str($p).unwrap();
            let dependencies = mock::deps![$($d),*].iter().cloned().collect();
            (package_id, dependencies)
        }};
    }

    pub(crate) use registry_entry;

    macro_rules! deps {
        [$($x:tt),* $(,)?] => (
            &[
                $($crate::core::registry::mock::dep!($x)),*
            ]
        );
    }

    pub(crate) use deps;

    macro_rules! dep {
        (($n:literal, $v:literal)) => {
            $crate::core::ManifestDependency::builder()
                .name($crate::core::PackageName::new($n))
                .version_req(::semver::VersionReq::parse($v).unwrap().into())
                .source_id($crate::core::SourceId::default_registry())
                .build()
        };

        (($n:literal, $v:literal, $s:literal)) => {
            $crate::core::ManifestDependency::builder()
                .name($crate::core::PackageName::new($n))
                .version_req(::semver::VersionReq::parse($v).unwrap().into())
                .source_id($crate::core::SourceId::from_display_str($s).unwrap())
                .build()
        };

        (($n:literal, $v:literal, (), $t:literal)) => {
            $crate::core::ManifestDependency::builder()
                .name($crate::core::PackageName::new($n))
                .version_req(::semver::VersionReq::parse($v).unwrap().into())
                .source_id($crate::core::SourceId::default_registry())
                .kind($crate::core::DepKind::Target($t.parse().unwrap()))
                .build()
        };

        (($n:literal, $v:literal, $s:literal, $t:literal)) => {
            $crate::core::ManifestDependency::builder()
                .name($crate::core::PackageName::new($n))
                .version_req(::semver::VersionReq::parse($v).unwrap().into())
                .source_id($crate::core::SourceId::from_display_str($s).unwrap())
                .kind($crate::core::DepKind::Target($t.parse().unwrap()))
                .build()
        };
    }

    pub(crate) use dep;

    macro_rules! pkgs {
        [$($x:expr),* $(,)?] => (
            &[
                $($crate::core::PackageId::from_display_str($x).unwrap()),*
            ]
        );
    }

    pub(crate) use pkgs;

    macro_rules! pkg_names {
        [$($x:tt),* $(,)?] => (
            &[
                $($crate::core::PackageName::new($x)),*
            ]
        );
    }

    pub(crate) use pkg_names;

    macro_rules! lock_entry {
        (($p:literal, [ $($d:tt),* $(,)? ] $(,)?)) => {{
            #[allow(unused_imports)]
            use $crate::core::registry::mock;
            let package_id = $crate::core::PackageId::from_display_str($p).unwrap();
            let dependencies: ::std::collections::BTreeSet<$crate::core::PackageName> = (
                mock::pkg_names![$($d),*].iter().cloned().collect()
            );
            $crate::core::lockfile::PackageLock::builder()
                .use_package_id(package_id)
                .dependencies(dependencies)
                .build()
        }};
    }

    pub(crate) use lock_entry;

    macro_rules! locks {
        [$($x:tt),* $(,)?] => (
            &[
                $($crate::core::registry::mock::lock_entry!($x)),*
            ]
        );
    }

    pub(crate) use locks;
}
