use std::collections::{BTreeMap, HashSet};
use std::fs;

use anyhow::{bail, ensure, Context, Result};
use camino::{Utf8Path, Utf8PathBuf};
use semver::{Version, VersionReq};
use serde::{Deserialize, Serialize};
use smol_str::SmolStr;
use toml::Value;
use tracing::trace;
use url::Url;

use crate::core::manifest::{
    ExternalTargetKind, LibTargetKind, ManifestDependency, ManifestMetadata, Summary, Target,
    TargetKind,
};
use crate::core::package::PackageId;
use crate::core::source::{GitReference, SourceId};
use crate::core::PackageName;
use crate::internal::fsx;
use crate::internal::fsx::PathUtf8Ext;
use crate::internal::to_version::ToVersion;

use super::Manifest;

/// This type is used to deserialize `Scarb.toml` files.
#[derive(Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct TomlManifest {
    pub package: Option<Box<TomlPackage>>,
    pub dependencies: Option<BTreeMap<PackageName, TomlDependency>>,
    pub lib: Option<TomlLibTarget>,
    pub target: Option<BTreeMap<TomlTargetKindName, Vec<TomlExternalTarget>>>,
}

/// Represents the `package` section of a `Scarb.toml`.
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct TomlPackage {
    pub name: PackageName,
    pub version: Version,
    pub authors: Option<Vec<String>>,
    pub urls: Option<BTreeMap<String, String>>,
    pub description: Option<String>,
    pub documentation: Option<String>,
    pub homepage: Option<String>,
    pub keywords: Option<Vec<String>>,
    pub license: Option<String>,
    pub license_file: Option<String>,
    pub readme: Option<String>,
    pub repository: Option<String>,
    pub metadata: Option<BTreeMap<String, String>>,
    /// **UNSTABLE** This package does not depend on Cairo's `core`.
    pub no_core: Option<bool>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum TomlDependency {
    /// [`VersionReq`] specified as a string, eg. `package = "<version>"`.
    Simple(VersionReq),
    /// Detailed specification as a table, eg. `package = { version = "<version>" }`.
    Detailed(DetailedTomlDependency),
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct DetailedTomlDependency {
    pub version: Option<VersionReq>,

    /// Relative to the file it appears in.
    pub path: Option<Utf8PathBuf>,

    pub git: Option<Url>,
    pub branch: Option<String>,
    pub tag: Option<String>,
    pub rev: Option<String>,
}

#[derive(Clone, Debug, Ord, PartialOrd, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(into = "SmolStr", try_from = "SmolStr")]
pub struct TomlTargetKindName(SmolStr);

impl TomlTargetKindName {
    pub fn try_new(name: SmolStr) -> Result<Self> {
        ensure!(&name != "lib", "target kind `lib` is reserved");
        Ok(Self(name))
    }

    pub fn to_smol_str(&self) -> SmolStr {
        self.0.clone()
    }

    pub fn into_smol_str(self) -> SmolStr {
        self.0
    }
}

impl From<TomlTargetKindName> for SmolStr {
    fn from(value: TomlTargetKindName) -> Self {
        value.into_smol_str()
    }
}

impl TryFrom<SmolStr> for TomlTargetKindName {
    type Error = anyhow::Error;

    fn try_from(value: SmolStr) -> Result<Self> {
        TomlTargetKindName::try_new(value)
    }
}

#[derive(Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct TomlLibTarget {
    /// The name of the target.
    ///
    /// Defaults to package name.
    pub name: Option<SmolStr>,

    /// Enable Sierra code generation.
    ///
    /// Defaults to `true`.
    pub sierra: Option<bool>,

    /// Enable CASM code generation.
    ///
    /// Defaults to `false`.
    pub casm: Option<bool>,
}

#[derive(Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct TomlExternalTarget {
    /// The name of the target.
    ///
    /// Defaults to package name.
    pub name: Option<SmolStr>,

    #[serde(flatten)]
    pub params: BTreeMap<SmolStr, Value>,
}

impl TomlManifest {
    pub fn read_from_path(path: &Utf8Path) -> Result<Self> {
        let contents = fs::read_to_string(path)
            .with_context(|| format!("failed to read manifest at `{path}`"))?;

        Self::read_from_str(&contents)
            .with_context(|| format!("failed to parse manifest at `{path}`"))
    }

    pub fn read_from_str(contents: &str) -> Result<Self> {
        toml::from_str(contents).map_err(Into::into)
    }
}

impl TomlDependency {
    fn resolve(&self) -> DetailedTomlDependency {
        match self {
            TomlDependency::Simple(version) => DetailedTomlDependency {
                version: Some(version.clone()),
                ..Default::default()
            },
            TomlDependency::Detailed(detailed) => detailed.clone(),
        }
    }
}

impl TomlManifest {
    pub fn to_manifest(&self, manifest_path: &Utf8Path, source_id: SourceId) -> Result<Manifest> {
        let Some(package) = self.package.as_deref() else {
            bail!("no `package` section found");
        };

        let package_id = {
            let name = package.name.clone();
            let version = package.version.clone().to_version()?;
            PackageId::new(name, version, source_id)
        };

        let mut dependencies = Vec::new();
        for (name, toml_dep) in self.dependencies.iter().flatten() {
            dependencies.push(toml_dep.to_dependency(name.clone(), manifest_path)?);
        }

        let no_core = package.no_core.unwrap_or(false);

        let targets = self.collect_targets(package.name.to_smol_str())?;

        Ok(Manifest {
            summary: Summary::build(package_id)
                .with_dependencies(dependencies)
                .no_core(no_core)
                .finish(),
            targets,
            metadata: ManifestMetadata {
                authors: package.authors.clone(),
                urls: package.urls.clone(),
                custom_metadata: package.metadata.clone(),
                description: package.description.clone(),
                documentation: package.documentation.clone(),
                homepage: package.homepage.clone(),
                keywords: package.keywords.clone(),
                license: package.license.clone(),
                license_file: package.license_file.clone(),
                readme: package.readme.clone(),
                repository: package.repository.clone(),
            },
        })
    }

    fn collect_targets(&self, package_name: SmolStr) -> Result<Vec<Target>> {
        let mut targets = Vec::new();

        if let Some(lib_toml) = &self.lib {
            let mut kind = LibTargetKind::default();
            if let Some(sierra) = lib_toml.sierra {
                kind.sierra = sierra;
            }
            if let Some(casm) = lib_toml.casm {
                kind.casm = casm;
            }

            let kind = TargetKind::Lib(kind);

            let name = lib_toml
                .name
                .clone()
                .unwrap_or_else(|| package_name.clone());

            let target = Target::new(name, kind);
            targets.push(target);
        }

        for (kind_name_toml, ext_toml) in self
            .target
            .iter()
            .flatten()
            .flat_map(|(k, vs)| vs.iter().map(|v| (k.clone(), v)))
        {
            let kind = ExternalTargetKind {
                kind_name: kind_name_toml.to_smol_str(),
                params: ext_toml.params.clone(),
            };
            let kind = TargetKind::External(kind);

            let name = ext_toml
                .name
                .clone()
                .unwrap_or_else(|| package_name.clone());

            let target = Target::new(name, kind);
            targets.push(target);
        }

        Self::check_unique_targets(&targets, &package_name)?;

        if targets.is_empty() {
            trace!("manifest has no targets, assuming default `lib` target");
            let kind = TargetKind::Lib(LibTargetKind::default());
            let target = Target::new(package_name, kind);
            targets.push(target);
        }

        Ok(targets)
    }

    fn check_unique_targets(targets: &[Target], package_name: &str) -> Result<()> {
        let mut used = HashSet::with_capacity(targets.len());
        for target in targets {
            if !used.insert((target.kind.name(), target.name.as_str())) {
                if target.name == package_name {
                    bail!(
                        "manifest contains duplicate target definitions `{}`, \
                        consider explicitly naming targets with the `name` field",
                        target.kind.name()
                    )
                } else {
                    bail!(
                        "manifest contains duplicate target definitions `{} ({})`, \
                        use different target names to resolve the conflict",
                        target.kind.name(),
                        target.name
                    )
                }
            }
        }
        Ok(())
    }
}

impl TomlDependency {
    fn to_dependency(
        &self,
        name: PackageName,
        manifest_path: &Utf8Path,
    ) -> Result<ManifestDependency> {
        self.resolve().to_dependency(name, manifest_path)
    }
}

impl DetailedTomlDependency {
    fn to_dependency(
        &self,
        name: PackageName,
        manifest_path: &Utf8Path,
    ) -> Result<ManifestDependency> {
        let version_req = self.version.to_owned().unwrap_or(VersionReq::STAR);

        if self.branch.is_some() || self.tag.is_some() || self.rev.is_some() {
            ensure!(
                self.git.is_some(),
                "dependency ({name}) is non-Git, but provides `branch`, `tag` or `rev`"
            );

            ensure!(
                [&self.branch, &self.tag, &self.rev]
                    .iter()
                    .filter(|o| o.is_some())
                    .count()
                    <= 1,
                "dependency ({name}) specification is ambiguous, \
                only one of `branch`, `tag` or `rev` is allowed"
            );
        }

        let source_id = match (self.version.as_ref(), self.git.as_ref(), self.path.as_ref()) {
            (None, None, None) => bail!(
                "dependency ({name}) must be specified providing a local path, Git repository, \
                or version to use"
            ),

            (_, Some(_), Some(_)) => bail!(
                "dependency ({name}) specification is ambiguous, \
                only one of `git` or `path` is allowed"
            ),

            (_, None, Some(path)) => {
                let root = manifest_path
                    .parent()
                    .expect("manifest path must always have parent");
                let path = root.join(path);
                let path = fsx::canonicalize(path)?;
                let path = path.try_as_utf8()?;
                SourceId::for_path(path)?
            }

            (_, Some(git), None) => {
                let reference = if let Some(branch) = &self.branch {
                    GitReference::Branch(branch.into())
                } else if let Some(tag) = &self.tag {
                    GitReference::Tag(tag.into())
                } else if let Some(rev) = &self.rev {
                    GitReference::Rev(rev.into())
                } else {
                    GitReference::DefaultBranch
                };

                SourceId::for_git(git, &reference)?
            }

            (Some(_), None, None) => SourceId::default_registry(),
        };

        Ok(ManifestDependency {
            name,
            version_req,
            source_id,
        })
    }
}
