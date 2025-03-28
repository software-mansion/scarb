use std::collections::BTreeSet;
use std::str::FromStr;

use anyhow::{Context, Result, anyhow};
use semver::Version;
use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};
use toml_edit::DocumentMut;
use typed_builder::TypedBuilder;

use crate::core::{Checksum, ManifestDependency, PackageId, PackageName, Resolve, SourceId};

const HEADER: &str = "# Code generated by scarb DO NOT EDIT.";

#[derive(
    Default, PartialEq, Eq, Clone, Copy, Debug, PartialOrd, Ord, Serialize_repr, Deserialize_repr,
)]
#[repr(u8)]
pub enum LockVersion {
    #[default]
    V1 = 1,
}

#[derive(Debug, Clone, Eq, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Lockfile {
    pub version: LockVersion,
    #[serde(rename = "package")]
    #[serde(default = "BTreeSet::new")]
    #[serde(skip_serializing_if = "BTreeSet::is_empty")]
    pub packages: BTreeSet<PackageLock>,
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize, TypedBuilder)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub struct PackageLock {
    pub name: PackageName,
    pub version: Version,

    #[builder(default)]
    #[serde(skip_serializing_if = "skip_path_source_id")]
    pub source: Option<SourceId>,

    #[builder(default)]
    pub checksum: Option<Checksum>,

    #[builder(default, setter(into))]
    #[serde(default = "BTreeSet::new")]
    #[serde(skip_serializing_if = "BTreeSet::is_empty")]
    pub dependencies: BTreeSet<PackageName>,
}

fn skip_path_source_id(sid: &Option<SourceId>) -> bool {
    sid.map(|sid| sid.is_path()).unwrap_or(true)
}

impl Lockfile {
    pub fn new(packages: impl IntoIterator<Item = PackageLock>) -> Self {
        Self {
            version: Default::default(),
            packages: packages.into_iter().collect(),
        }
    }

    pub fn from_resolve(resolve: &Resolve) -> Self {
        let include_package = |package_id: &PackageId| !package_id.source_id.is_std();
        let packages = resolve
            .package_ids()
            .filter(include_package)
            .map(|package| {
                let deps = resolve
                    .package_dependencies(package)
                    .filter(include_package)
                    .map(|dep| dep.name.clone())
                    .collect::<BTreeSet<_>>();

                let summary = &resolve.summaries[&package];

                PackageLock::builder()
                    .use_package_id(package)
                    .dependencies(deps)
                    .checksum(summary.checksum.clone())
                    .build()
            });
        Self::new(packages)
    }

    pub fn packages(&self) -> impl Iterator<Item = &PackageLock> {
        self.packages.iter()
    }

    pub fn packages_matching(&self, dependency: ManifestDependency) -> Option<Result<PackageId>> {
        self.packages()
            .filter(|p| dependency.matches_name_and_version(&p.name, &p.version))
            .find(|p| {
                p.source
                    .map(|sid| sid.can_lock_source_id(dependency.source_id))
                    // No locking occurs on path sources.
                    .unwrap_or(false)
            })
            .cloned()
            .map(|p| p.try_into())
    }

    fn body(&self) -> Result<DocumentMut> {
        let doc = toml_edit::ser::to_string_pretty(self)?;
        let mut doc = doc.parse::<DocumentMut>()?;

        for packages in doc["package"].as_array_of_tables_mut().iter_mut() {
            for pkg in packages.iter_mut() {
                if let Some(deps) = pkg.get_mut("dependencies") {
                    if let Some(deps) = deps.as_array_mut() {
                        deps.iter_mut().for_each(|dep| {
                            dep.decor_mut().set_prefix("\n ");
                        });
                        if deps.len() > 1 {
                            deps.set_trailing("\n");
                        } else {
                            deps.set_trailing(",\n");
                        }
                    }
                }
            }
        }

        Ok(doc)
    }

    pub fn render(&self) -> Result<String> {
        Ok(format!("{HEADER}\n{}", self.body()?))
    }
}

impl FromStr for Lockfile {
    type Err = anyhow::Error;

    fn from_str(content: &str) -> Result<Self> {
        if content.is_empty() {
            Ok(Self::default())
        } else {
            toml::from_str(content).context("failed to parse lockfile content")
        }
    }
}

type UsePackageIdFields = ((PackageName,), (Version,), (Option<SourceId>,), (), ());
impl PackageLockBuilder {
    pub fn use_package_id(self, package_id: PackageId) -> PackageLockBuilder<UsePackageIdFields> {
        self.name(package_id.name.clone())
            .version(package_id.version.clone())
            .source(Some(package_id.source_id))
    }
}

impl TryFrom<PackageLock> for PackageId {
    type Error = anyhow::Error;

    fn try_from(value: PackageLock) -> Result<Self> {
        let source_id = value.source.ok_or_else(|| {
            anyhow!(
                "missing source id in package lock for package {}",
                value.name
            )
        })?;
        Ok(Self::new(value.name, value.version, source_id))
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use expect_test::expect;
    use semver::Version;

    use crate::core::lockfile::{Lockfile, PackageLock};
    use crate::core::{Checksum, PackageName, SourceId};

    #[test]
    fn simple() {
        let checksum = Checksum::parse(
            "sha256:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
        )
        .unwrap();

        let pkg1 = PackageLock::builder()
            .name(PackageName::new("first"))
            .version(Version::parse("1.0.0").unwrap())
            .source(Some(SourceId::default_registry()))
            .checksum(Some(checksum))
            .dependencies([PackageName::new("fourth")])
            .build();

        let pkg2 = PackageLock::builder()
            .name(PackageName::new("second"))
            .version(Version::parse("1.0.0").unwrap())
            .dependencies([PackageName::new("fourth")])
            .build();

        let pkg3 = PackageLock::builder()
            .name(PackageName::new("third"))
            .version(Version::parse("2.1.0").unwrap())
            .source(Some(SourceId::mock_git()))
            .build();

        let pkg4 = PackageLock::builder()
            .name(PackageName::new("fourth"))
            .version(Version::parse("80.0.85").unwrap())
            .source(Some(SourceId::default_registry()))
            .dependencies([PackageName::new("third")])
            .build();

        let lock = Lockfile::new(vec![pkg1, pkg2, pkg3, pkg4]);

        let serialized = expect![[r#"
            # Code generated by scarb DO NOT EDIT.
            version = 1

            [[package]]
            name = "first"
            version = "1.0.0"
            source = "registry+https://scarbs.xyz/"
            checksum = "sha256:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
            dependencies = [
             "fourth",
            ]

            [[package]]
            name = "fourth"
            version = "80.0.85"
            source = "registry+https://scarbs.xyz/"
            dependencies = [
             "third",
            ]

            [[package]]
            name = "second"
            version = "1.0.0"
            dependencies = [
             "fourth",
            ]

            [[package]]
            name = "third"
            version = "2.1.0"
            source = "git+https://github.com/starkware-libs/cairo.git?tag=test"
        "#]];

        serialized.assert_eq(&lock.render().unwrap());
        let deserialized = Lockfile::from_str(serialized.data()).unwrap();
        assert_eq!(lock, deserialized);
    }

    #[test]
    fn empty() {
        let lock = Lockfile::new([]);

        let serialized = "# Code generated by scarb DO NOT EDIT.\nversion = 1\n";
        assert_eq!(serialized, lock.render().unwrap());

        let deserialized = Lockfile::from_str(serialized).unwrap();
        assert_eq!(lock, deserialized);
    }
}
