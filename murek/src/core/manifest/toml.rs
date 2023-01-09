use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use semver::{Version, VersionReq};
use serde::{Deserialize, Serialize};
use smol_str::SmolStr;
use url::Url;

use crate::core::manifest::{ManifestDependency, ManifestMetadata, Summary};
use crate::core::package::PackageId;
use crate::core::restricted_names::validate_package_name;
use crate::core::source::{GitReference, SourceId};
use crate::internal::fsx;

use super::Manifest;

/// This type is used to deserialize `Murek.toml` files.
#[derive(Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct TomlManifest {
    pub package: Option<Box<TomlPackage>>,
    pub dependencies: Option<BTreeMap<SmolStr, TomlDependency>>,
}

/// Represents the `package` section of a `Murek.toml`.
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct TomlPackage {
    pub name: SmolStr,
    pub version: Version,
    pub authors: Option<Vec<String>>,
    pub custom_links: Option<BTreeMap<String, Url>>,
    pub custom_metadata: Option<BTreeMap<String, String>>,
    pub description: Option<String>,
    pub documentation: Option<Url>,
    pub homepage: Option<Url>,
    pub keywords: Option<Vec<String>>,
    pub license: Option<String>,
    pub license_file: Option<PathBuf>,
    pub readme: Option<PathBuf>,
    pub repository: Option<Url>,
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
    pub path: Option<PathBuf>,

    pub git: Option<Url>,
    pub branch: Option<String>,
    pub tag: Option<String>,
    pub rev: Option<String>,
}

impl TomlManifest {
    pub fn read_from_path(path: &Path) -> Result<Self> {
        let contents = fs::read_to_string(path)
            .with_context(|| format!("failed to read manifest at `{}`", path.display()))?;

        Self::read_from_str(&contents)
            .with_context(|| format!("failed to parse manifest at `{}`", path.display()))
    }

    pub fn read_from_str(contents: &str) -> Result<Self> {
        toml_edit::easy::from_str(contents).map_err(Into::into)
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
    pub fn to_manifest(&self, manifest_path: &Path, source_id: SourceId) -> Result<Manifest> {
        let Some(package) = self.package.as_deref() else {
            bail!("no `package` section found");
        };

        let package_id = {
            let name = package.name.clone();
            validate_package_name(&name, "package name")?;

            let version = package.version.clone();

            PackageId::pure(name, version, source_id)
        };

        let mut dependencies = Vec::new();
        for (name, toml_dep) in self.dependencies.iter().flatten() {
            dependencies.push(toml_dep.to_dependency(name, manifest_path)?);
        }

        Ok(Manifest {
            summary: Summary::new(package_id, dependencies),
            metadata: ManifestMetadata {
                authors: package.authors.clone(),
                custom_links: package.custom_links.clone(),
                custom_metadata: package.custom_metadata.clone(),
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
}

impl TomlDependency {
    fn to_dependency(&self, name: &str, manifest_path: &Path) -> Result<ManifestDependency> {
        self.resolve().to_dependency(name, manifest_path)
    }
}

impl DetailedTomlDependency {
    fn to_dependency(&self, name: &str, manifest_path: &Path) -> Result<ManifestDependency> {
        validate_package_name(name, "dependency name")?;

        let version_req = self.version.to_owned().unwrap_or(VersionReq::STAR);

        if self.branch.is_some() || self.tag.is_some() || self.rev.is_some() {
            if self.git.is_none() {
                bail!("dependency ({name}) is non-Git, but provides `branch`, `tag` or `rev`");
            }

            let n_refs = [&self.branch, &self.tag, &self.rev]
                .iter()
                .filter(|o| o.is_some())
                .count();
            if n_refs > 1 {
                bail!(
                    "dependency ({name}) specification is ambiguous, \
                    only one of `branch`, `tag` or `rev` is allowed"
                );
            }
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
                SourceId::for_path(&path)?
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

            (Some(_), None, None) => todo!("Registry sources are not implemented yet."),
        };

        Ok(ManifestDependency {
            name: name.into(),
            version_req,
            source_id,
        })
    }
}
