use std::fmt;

use anyhow::{bail, ensure, Result};
use camino::{Utf8Path, Utf8PathBuf};
use indoc::{formatdoc, indoc};

use scarb_metadata::{Metadata, PackageMetadata};

/// [`clap`] structured arguments that provide package selection.
///
/// ## Usage
///
/// ```no_run
/// # use scarb_ui::args::PackagesFilter;
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

impl PackagesFilter {
    /// Find *exactly one* package matching the filter.
    ///
    /// Returns an error if no or more than one packages were found.
    pub fn match_one<S: PackagesSource>(&self, source: &S) -> Result<S::Package> {
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
            bail!(indoc! {r#"
                could not determine which package to work on
                help: use the `--package` option to specify the package
            "#});
        }

        let found = Self::do_match::<S>(&spec, self.workspace, members.into_iter())?;

        ensure!(
            found.len() <= 1,
            formatdoc! {r#"
                workspace has multiple members matching `{spec}`
                help: use the `--package` option to specify single package
            "#}
        );

        Ok(found.into_iter().next().unwrap())
    }

    /// Find *at least one* package matching the filter.
    ///
    /// Returns an error if no packages were found.
    pub fn match_many<S: PackagesSource>(&self, source: &S) -> Result<Vec<S::Package>> {
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

    fn current_package<S: PackagesSource>(&self, source: &S) -> Result<Option<S::Package>> {
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
    ) -> Result<Vec<S::Package>> {
        let mut members = members.peekable();

        ensure!(members.peek().is_some(), "workspace has no members");

        let matches = if workspace {
            members.collect::<Vec<_>>()
        } else {
            members
                .filter(|pkg| spec.matches(S::package_name_of(pkg)))
                .collect::<Vec<_>>()
        };

        if matches.is_empty() {
            match spec {
                Spec::One(package_name) => bail!("package `{package_name}` not found in workspace"),
                Spec::All | Spec::Glob(_) => bail!("no workspace members match `{spec}`"),
            }
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
    fn parse(string: &'a str) -> Result<Self> {
        let string = string.trim();

        if !string.contains('*') {
            return Ok(Self::One(string));
        }

        ensure!(
            string.chars().filter(|c| *c == '*').count() == 1,
            "invalid package spec: * character can only occur once in the pattern"
        );
        ensure!(
            string.ends_with('*'),
            "invalid package spec: only `prefix*` patterns are allowed"
        );

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

impl<'a> fmt::Display for Spec<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Spec::All => write!(f, "*"),
            Spec::One(name) => write!(f, "{name}"),
            Spec::Glob(pat) => write!(f, "{pat}*"),
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

impl WithManifestPath for PackageMetadata {
    fn manifest_path(&self) -> &Utf8Path {
        &self.manifest_path
    }
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
        self.runtime_manifest.clone()
    }
}
