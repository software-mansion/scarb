use anyhow::{Context, Result};
use camino::Utf8Path;
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use toml_edit::{Array, Document, Item, Value};

use crate::core::{PackageId, PackageName, Resolve};
use crate::internal::fsx;

#[derive(Default, PartialEq, Eq, Clone, Copy, Debug, PartialOrd, Ord, Serialize, Deserialize)]
pub enum LockVersion {
    #[serde(rename = "1")]
    #[default]
    V1 = 1,
}

#[derive(Debug, Eq, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct Lockfile {
    pub version: LockVersion,
    #[serde(rename = "package")]
    #[serde(default = "BTreeSet::new")]
    #[serde(skip_serializing_if = "BTreeSet::is_empty")]
    pub packages: BTreeSet<PackageLock>,
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
#[serde(from = "serde_ext::PackageLock", into = "serde_ext::PackageLock")]
pub struct PackageLock {
    pub id: PackageId,
    pub dependencies: BTreeSet<PackageName>,
}

mod serde_ext {
    use crate::core::{PackageName, SourceId};
    use semver::Version;
    use serde::{Deserialize, Serialize};
    use std::collections::BTreeSet;

    #[derive(Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
    #[serde(rename_all = "snake_case")]
    pub struct PackageLock {
        pub name: PackageName,
        pub version: Version,
        #[serde(default = "SourceId::empty_path")]
        #[serde(skip_serializing_if = "SourceId::is_path")]
        pub source: SourceId,
        #[serde(default = "BTreeSet::new")]
        #[serde(skip_serializing_if = "BTreeSet::is_empty")]
        pub dependencies: BTreeSet<PackageName>,
    }
}

impl From<serde_ext::PackageLock> for PackageLock {
    fn from(value: serde_ext::PackageLock) -> Self {
        Self {
            dependencies: value.dependencies,
            id: PackageId::new(value.name, value.version, value.source),
        }
    }
}

impl From<PackageLock> for serde_ext::PackageLock {
    fn from(value: PackageLock) -> Self {
        Self {
            dependencies: value.dependencies,
            name: value.id.name.clone(),
            version: value.id.version.clone(),
            source: value.id.source_id,
        }
    }
}

impl Lockfile {
    pub fn new(packages: impl Iterator<Item = PackageLock>) -> Self {
        Self {
            version: Default::default(),
            packages: packages.collect(),
        }
    }

    pub fn from_resolve(resolve: &Resolve) -> Self {
        let packages = resolve.package_ids().map(|package| {
            let deps = resolve
                .package_dependencies(package)
                .map(|dep| dep.name.clone());
            PackageLock::new(&package, deps)
        });
        Self::new(packages)
    }

    pub fn from_path(path: impl AsRef<Utf8Path>) -> Result<Self> {
        if path.as_ref().is_file() {
            let content = fsx::read_to_string(path.as_ref())
                .with_context(|| format!("Failed to read lockfile at {}", path.as_ref()))?;
            if content.is_empty() {
                Ok(Self::default())
            } else {
                content
                    .try_into()
                    .with_context(|| format!("Failed to parse lockfile at {}", path.as_ref()))
            }
        } else {
            Ok(Self::default())
        }
    }

    fn header(&self) -> String {
        "# Code generated by scarb DO NOT EDIT.".into()
    }

    fn body(&self) -> Result<Document> {
        let toml = toml::Table::try_from(self)?;
        let mut doc = toml.to_string().parse::<Document>()?;

        for packs in doc["package"].as_array_of_tables_mut().iter_mut() {
            for pack in packs.iter_mut() {
                if let Some(deps) = pack.remove("dependencies") {
                    let mut deps: Array = deps
                        .into_value()
                        .unwrap()
                        .as_array()
                        .unwrap()
                        .iter()
                        .map(|dep| {
                            let mut dep = dep.clone();
                            dep.decor_mut().set_prefix("\n ");
                            dep
                        })
                        .collect();
                    deps.set_trailing(",\n");
                    pack.insert("dependencies", Item::Value(Value::Array(deps)));
                }
            }
        }

        Ok(doc)
    }

    pub fn render(&self) -> Result<String> {
        Ok(format!("{}\n{}", self.header(), self.body()?))
    }
}

impl TryFrom<String> for Lockfile {
    type Error = anyhow::Error;

    fn try_from(value: String) -> Result<Self> {
        let value = value
            .lines()
            .skip_while(|line| *line != "# Code generated by scarb DO NOT EDIT.")
            .skip(1)
            .join("\n");
        Ok(toml::from_str(&value)?)
    }
}

impl PackageLock {
    pub fn new(package: &PackageId, dependencies: impl Iterator<Item = PackageName>) -> Self {
        Self {
            id: *package,
            dependencies: dependencies.collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::core::lockfile::{Lockfile, PackageLock};
    use crate::core::{PackageId, PackageName, SourceId};

    use core::default::Default;
    use indoc::indoc;
    use semver::Version;
    use snapbox::assert_eq;

    #[test]
    fn simple() {
        let pkg1 = PackageLock::new(
            &PackageId::new(
                PackageName::CORE,
                Version::parse("1.0.0").unwrap(),
                Default::default(),
            ),
            vec![PackageName::STARKNET, PackageName::new("locker")].into_iter(),
        );

        let pkg2 = PackageLock::new(
            &PackageId::new(
                PackageName::STARKNET,
                Version::parse("1.0.0").unwrap(),
                SourceId::empty_path(),
            ),
            vec![PackageName::CORE].into_iter(),
        );

        let pkg3 = PackageLock::new(
            &PackageId::new(
                PackageName::new("third"),
                Version::parse("2.1.0").unwrap(),
                SourceId::mock_git(),
            ),
            vec![].into_iter(),
        );

        let pkg4 = PackageLock::new(
            &PackageId::new(
                PackageName::new("fourth"),
                Version::parse("80.0.85").unwrap(),
                SourceId::for_std(),
            ),
            vec![].into_iter(),
        );

        let lock = Lockfile::new(vec![pkg1, pkg2, pkg3, pkg4].into_iter());

        let serialized = indoc! {r#"
            # Code generated by scarb DO NOT EDIT.
            version = "1"

            [[package]]
            name = "core"
            source = "registry+https://there-is-no-default-registry-yet.com/"
            version = "1.0.0"
            dependencies = [
             "locker",
             "starknet",
            ]

            [[package]]
            name = "fourth"
            source = "std"
            version = "80.0.85"

            [[package]]
            name = "starknet"
            version = "1.0.0"
            dependencies = [
             "core",
            ]

            [[package]]
            name = "third"
            source = "git+https://github.com/starkware-libs/cairo.git?tag=test"
            version = "2.1.0"
        "#};

        assert_eq(serialized, lock.render().unwrap());
        let deserialized: Lockfile = serialized.to_string().try_into().unwrap();
        assert_eq!(lock, deserialized);
    }

    #[test]
    fn empty() {
        let lock = Lockfile {
            version: Default::default(),
            packages: Default::default(),
        };

        let serialized = "# Code generated by scarb DO NOT EDIT.\nversion = \"1\"\n";
        assert_eq!(serialized, lock.render().unwrap());

        let deserialized: Lockfile = serialized.to_string().try_into().unwrap();
        assert_eq!(lock, deserialized);
    }
}
