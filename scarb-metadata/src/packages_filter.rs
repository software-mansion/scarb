//! [`clap`] arguments implementing Scarb-compatible package selection (`-p` flag etc.)

#![deprecated(
    since = "1.7.0",
    note = "This module has been moved to `scarb-ui` crate hosted in Scarb repository. \
    Removal from `scarb-metadata` is planned in when no usage will be present in open source projects."
)]

use camino::{Utf8Path, Utf8PathBuf};

use crate::{Metadata, PackageMetadata};

/// [`clap`] structured arguments that provide package selection.
///
/// ## Usage
///
/// ```no_run
/// # use scarb_metadata::packages_filter::PackagesFilter;
/// #[derive(clap::Parser)]
/// struct Args {
///     #[command(flatten)]
///     packages_filter: PackagesFilter,
/// }
/// ```
#[derive(clap::Parser, Clone, Debug)]
pub struct PackagesFilter {
    /// Packages to run this command on, can be a concrete package name (`foobar`) or
    /// a prefix glob (`foo*`).
    #[arg(short, long, value_name = "SPEC", default_value = "*")]
    package: String,
    /// Run for all packages in the workspace.
    #[arg(short, long, conflicts_with = "package")]
    workspace: bool,
}

/// Error type returned from [`PackagesFilter::match_one`] and [`PackagesFilter::match_many`]
/// functions.
///
/// Its internal structure is unspecified, but stringified messages convey meaningful information
/// to application users.
#[derive(Clone, Debug, thiserror::Error, Eq, PartialEq)]
#[error(transparent)]
pub struct Error(#[from] InnerError);

#[derive(Clone, Debug, thiserror::Error, Eq, PartialEq)]
enum InnerError {
    // Matching errors.
    #[error("package `{package_name}` not found in workspace")]
    OneNotFound { package_name: String },
    #[error("no workspace members match `{spec}`")]
    ManyNotFound { spec: String },
    #[error("workspace has no members")]
    WorkspaceHasNoMembers,
    #[error("could not determine which package to work on. Use the `--package` option to specify the package.")]
    CouldNotDeterminePackageToWorkOn,
    #[error("workspace has multiple members matching `{spec}`. Use the `--package` option to specify single package.")]
    FoundMultiple { spec: String },

    // Spec parsing errors.
    #[error("invalid package spec: * character can only occur once in the pattern")]
    MultipleStars,
    #[error("invalid package spec: only `prefix*` patterns are allowed")]
    NotPrefix,
}

impl InnerError {
    fn not_found(spec: &Spec<'_>) -> Self {
        match spec {
            Spec::One(package_name) => Self::OneNotFound {
                package_name: package_name.to_string(),
            },
            spec @ (Spec::All | Spec::Glob(_)) => Self::ManyNotFound {
                spec: spec.to_string(),
            },
        }
    }
}

impl PackagesFilter {
    /// Find *exactly one* package matching the filter.
    ///
    /// Returns an error if no or more than one packages were found.
    pub fn match_one<S: PackagesSource>(&self, source: &S) -> Result<S::Package, Error> {
        let spec = Spec::parse(&self.package)?;

        // Check for current package.
        // If none (in case of virtual workspace), run for all members.
        if self.current_selected(&spec) {
            if let Some(pkg) = self.current_package(source)? {
                return Ok(pkg);
            }
        }

        let members = source.members();
        if (self.workspace || matches!(spec, Spec::All)) && members.len() > 1 {
            return Err(InnerError::CouldNotDeterminePackageToWorkOn.into());
        }

        let found = Self::do_match::<S>(&spec, self.workspace, members.into_iter())?;

        if found.len() > 1 {
            return Err(InnerError::FoundMultiple {
                spec: spec.to_string(),
            }
            .into());
        }

        Ok(found.into_iter().next().unwrap())
    }

    /// Find *at least one* package matching the filter.
    ///
    /// Returns an error if no packages were found.
    pub fn match_many<S: PackagesSource>(&self, source: &S) -> Result<Vec<S::Package>, Error> {
        let spec = Spec::parse(&self.package)?;

        // Check for current package.
        // If none (in case of virtual workspace), run for all members.
        if self.current_selected(&spec) {
            if let Some(pkg) = self.current_package(source)? {
                return Ok(vec![pkg]);
            }
        }

        let members = source.members();
        Self::do_match::<S>(&spec, self.workspace, members.into_iter())
    }

    fn current_package<S: PackagesSource>(&self, source: &S) -> Result<Option<S::Package>, Error> {
        Ok(source
            .members()
            .iter()
            .find(|m| m.manifest_path() == source.runtime_manifest())
            .cloned())
    }

    fn current_selected(&self, spec: &Spec<'_>) -> bool {
        !self.workspace && matches!(spec, Spec::All)
    }

    fn do_match<S: PackagesSource>(
        spec: &Spec<'_>,
        workspace: bool,
        members: impl Iterator<Item = S::Package>,
    ) -> Result<Vec<S::Package>, Error> {
        let mut members = members.peekable();

        if members.peek().is_none() {
            return Err(InnerError::WorkspaceHasNoMembers.into());
        }

        let matches = if workspace {
            members.collect::<Vec<_>>()
        } else {
            members
                .filter(|pkg| spec.matches(S::package_name_of(pkg)))
                .collect::<Vec<_>>()
        };

        if matches.is_empty() {
            return Err(InnerError::not_found(spec).into());
        }

        Ok(matches)
    }
}

enum Spec<'a> {
    All,
    One(&'a str),
    Glob(&'a str),
}

impl<'a> Spec<'a> {
    fn parse(string: &'a str) -> Result<Self, InnerError> {
        let string = string.trim();

        if !string.contains('*') {
            return Ok(Self::One(string));
        }

        if string.chars().filter(|c| *c == '*').count() != 1 {
            return Err(InnerError::MultipleStars);
        }

        if !string.ends_with('*') {
            return Err(InnerError::NotPrefix);
        }

        let string = string.trim_end_matches('*');

        if string.is_empty() {
            Ok(Self::All)
        } else {
            Ok(Self::Glob(string))
        }
    }

    fn matches(&self, name: &str) -> bool {
        match self {
            Spec::All => true,
            Spec::One(pat) => name == *pat,
            Spec::Glob(pat) => name.starts_with(pat),
        }
    }
}

impl<'a> ToString for Spec<'a> {
    fn to_string(&self) -> String {
        match self {
            Spec::All => "*".to_owned(),
            Spec::One(name) => name.to_string(),
            Spec::Glob(pat) => format!("{pat}*"),
        }
    }
}

/// Generic interface used by [`PackagesSource`] to pull information from.
///
/// This trait is Scarb's internal implementation detail, **do not implement for your own types**.
pub trait WithManifestPath {
    #[doc(hidden)]
    fn manifest_path(&self) -> &Utf8Path;
}

/// Generic interface used by [`PackagesFilter`] to pull information from.
///
/// This trait is Scarb's internal implementation detail, **do not implement for your own types**.
/// Inside Scarb there are implementations for Scarb's core types, which allows Scarb to re-use
/// [`PackagesFilter`] logic.
pub trait PackagesSource {
    /// Type which represents a Scarb package in this source.
    type Package: Clone + WithManifestPath;

    #[doc(hidden)]
    fn package_name_of(package: &Self::Package) -> &str;

    #[doc(hidden)]
    fn members(&self) -> Vec<Self::Package>;

    #[doc(hidden)]
    fn runtime_manifest(&self) -> Utf8PathBuf;
}

impl PackagesSource for Metadata {
    type Package = PackageMetadata;

    #[inline(always)]
    fn package_name_of(package: &Self::Package) -> &str {
        &package.name
    }

    #[inline(always)]
    fn members(&self) -> Vec<Self::Package> {
        self.packages
            .iter()
            .filter(|pkg| self.workspace.members.contains(&pkg.id))
            .cloned()
            .collect()
    }

    fn runtime_manifest(&self) -> Utf8PathBuf {
        let path = self.runtime_manifest.clone();
        if !path.as_str().is_empty() {
            path
        } else {
            self.workspace.manifest_path.clone()
        }
    }
}
