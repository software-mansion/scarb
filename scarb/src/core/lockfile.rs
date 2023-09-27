use anyhow::{bail, Result};
use itertools::Itertools;
use semver::Version;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::io::Write;
use toml_edit::{Array, Document, Item, Value};

use crate::core::{Config, PackageId, PackageName, Resolve, SourceId};

#[derive(Default, PartialEq, Eq, Clone, Copy, Debug, PartialOrd, Ord, Serialize, Deserialize)]
pub enum LockVersion {
    #[serde(rename = "1")]
    #[default]
    V1 = 1,
}

#[derive(Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct Lockfile {
    pub version: LockVersion,
    #[serde(rename = "package")]
    #[serde(default = "BTreeSet::new")]
    #[serde(skip_serializing_if = "BTreeSet::is_empty")]
    pub packages: BTreeSet<PackageLock>,
}

#[derive(Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct PackageLock {
    pub name: PackageName,
    pub version: Version,
    #[serde(default = "SourceId::empty_path")]
    #[serde(skip_serializing_if = "is_path")]
    pub source: SourceId,
    #[serde(default = "BTreeSet::new")]
    #[serde(skip_serializing_if = "BTreeSet::is_empty")]
    pub dependencies: BTreeSet<PackageName>,
}

fn is_path(src: &SourceId) -> bool {
    src.is_path()
}

impl Lockfile {
    pub fn new(resolve: &Resolve) -> Self {
        let mut lock = Self {
            version: Default::default(),
            packages: Default::default(),
        };
        for package in resolve.graph.nodes() {
            let mut plock = PackageLock::new(&package);
            for dependency in resolve
                .graph
                .neighbors_directed(package, petgraph::Direction::Outgoing)
            {
                plock
                    .add_dependency(dependency.name.clone())
                    .unwrap_or_else(|_| todo!());
            }
            lock.add_package(plock).unwrap_or_else(|_| todo!());
        }
        lock
    }

    pub fn add_package(&mut self, pack: PackageLock) -> Result<()> {
        if self.packages.insert(pack) {
            Ok(())
        } else {
            bail!("package has already been added")
        }
    }

    fn generate_lock_string(&self) -> Result<String> {
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
        let mut res = String::new();

        let first_line = "# Code generated by scarb DO NOT EDIT.\n";
        res.push_str(first_line);
        res.push_str(&doc.to_string());
        Ok(res)
    }

    pub fn generate_lockfile(&self, config: &Config) -> Result<()> {
        let lock_str = self.generate_lock_string()?;
        let lockfile = config
            .manifest_path()
            .parent()
            .expect("failed to get the parent path of the manifest")
            .join("Scarb.lock");
        let mut f = std::fs::File::create(lockfile)?;
        f.write_all(lock_str.as_ref())?;
        Ok(())
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
    pub fn new(package: &PackageId) -> Self {
        Self {
            name: package.name.clone(),
            version: package.version.clone(),
            source: package.source_id,
            dependencies: Default::default(),
        }
    }

    pub fn add_dependency(&mut self, dep: PackageName) -> Result<()> {
        if self.dependencies.insert(dep) {
            Ok(())
        } else {
            bail!("dependency has already been added")
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::core::lockfile::{Lockfile, PackageLock};
    use crate::core::{PackageName, SourceId};
    use core::default::Default;
    use semver::Version;
    use snapbox::assert_eq;

    fn prep_simple_lock() -> Lockfile {
        let mut plock = PackageLock {
            name: PackageName::CORE,
            version: Version::parse("1.0.0").unwrap(),
            source: Default::default(),
            dependencies: Default::default(),
        };
        plock.add_dependency(PackageName::STARKNET).unwrap();
        plock.add_dependency(PackageName::new("locker")).unwrap();
        let mut lock = Lockfile {
            version: Default::default(),
            packages: Default::default(),
        };
        lock.add_package(plock).unwrap();
        let mut packa = PackageLock {
            name: PackageName::STARKNET,
            version: Version::parse("1.0.0").unwrap(),
            source: SourceId::empty_path(),
            dependencies: Default::default(),
        };
        packa.add_dependency(PackageName::CORE).unwrap();
        lock.add_package(packa).unwrap();
        lock.add_package(PackageLock {
            name: PackageName::new("empty"),
            version: Version::parse("4.2.0").unwrap(),
            source: SourceId::mock_git(),
            dependencies: Default::default(),
        })
        .unwrap();
        lock.add_package(PackageLock {
            name: PackageName::new("std"),
            version: Version::parse("80.0.85").unwrap(),
            source: SourceId::for_std(),
            dependencies: Default::default(),
        })
        .unwrap();
        lock
    }

    #[test]
    fn simple_serde() {
        //
        let s = r#"# Code generated by scarb DO NOT EDIT.
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
name = "empty"
source = "git+https://github.com/starkware-libs/cairo.git?tag=test"
version = "4.2.0"

[[package]]
name = "starknet"
version = "1.0.0"
dependencies = [
 "core",
]

[[package]]
name = "std"
source = "std"
version = "80.0.85"
"#;
        let ori_lock = prep_simple_lock();
        assert_eq(s, ori_lock.generate_lock_string().unwrap());
        let lock: Lockfile = s.to_string().try_into().unwrap();
        assert_eq!(ori_lock, lock);
    }

    #[test]
    fn test_empty() {
        let ori_lock = Lockfile {
            version: Default::default(),
            packages: Default::default(),
        };
        let s = "# Code generated by scarb DO NOT EDIT.\nversion = \"1\"\n";
        assert_eq!(s, ori_lock.generate_lock_string().unwrap());

        let lock: Lockfile = s.to_string().try_into().unwrap();
        assert_eq!(ori_lock, lock);
    }
}
