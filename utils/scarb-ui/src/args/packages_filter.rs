use std::collections::HashSet;
use std::ffi::OsString;
use std::fmt;

use anyhow::{bail, ensure, Result};
use camino::{Utf8Path, Utf8PathBuf};

use scarb_metadata::{Metadata, PackageMetadata};

const PACKAGES_FILTER_DELIMITER: char = ',';

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
    #[arg(
        short,
        long,
        default_value = "*",
        value_delimiter = PACKAGES_FILTER_DELIMITER,
        value_name = "SPEC",
        env = "SCARB_PACKAGES_FILTER"
    )]
    package: Vec<String>,
    /// Run for all packages in the workspace.
    #[arg(short, long, conflicts_with = "package")]
    workspace: bool,
}

impl PackagesFilter {
    /// Find *exactly one* package matching the filter.
    ///
    /// Returns an error if no or more than one packages were found.
    pub fn match_one<S: PackagesSource>(&self, source: &S) -> Result<S::Package> {
        let specs = self.package_specs()?;

        // Check for current package.
        // If none (in case of virtual workspace), run for all members.
        if self.current_selected(&specs) {
            if let Some(pkg) = self.current_package(source)? {
                return Ok(pkg);
            }
        }

        let members = source.members();

        if (self.workspace || specs.iter().any(|spec| matches!(spec, Spec::All)))
            && members.len() > 1
        {
            bail!(
                "could not determine which package to work on\n\
                help: use the `--package` option to specify the package"
            );
        }

        let specs_filter: String = specs
            .iter()
            .map(|s| s.to_string())
            .collect::<Vec<_>>()
            .join(PACKAGES_FILTER_DELIMITER.to_string().as_str());
        let found = Self::do_match_all::<S>(specs, self.workspace, members)?;

        ensure!(
            found.len() <= 1,
            "workspace has multiple members matching `{specs_filter}`\n\
            help: use the `--package` option to specify single package"
        );

        Ok(found.into_iter().next().unwrap())
    }

    /// Find *at least one* package matching the filter.
    ///
    /// Returns an error if no packages were found.
    pub fn match_many<S: PackagesSource>(&self, source: &S) -> Result<Vec<S::Package>> {
        let specs = self.package_specs()?;

        // Check for current package.
        // If none (in case of virtual workspace), run for all members.
        if self.current_selected(&specs) {
            if let Some(pkg) = self.current_package(source)? {
                return Ok(vec![pkg]);
            }
        }

        let members = source.members();
        Self::do_match_all::<S>(specs, self.workspace, members)
    }

    /// Generate a new [`PackagesFilter`] for the given slice  of packages.
    ///
    /// This is useful when you want to build an env filter from matched packages.
    /// See [`PackagesFilter::to_env`] for more details.
    pub fn generate_for<'a, S: PackagesSource>(
        packages: impl Iterator<Item = &'a S::Package>,
    ) -> Self
    where
        S::Package: 'a,
    {
        let names: Vec<String> = packages
            .map(|p| S::package_name_of(p).to_string())
            .collect();
        Self {
            package: names,
            workspace: false,
        }
    }

    /// Get the packages filter as an [`OsString`].
    ///
    /// This value can be passed as `SCARB_PACKAGES_FILTER` variable to child processes.
    pub fn to_env(self) -> OsString {
        self.package
            .join(PACKAGES_FILTER_DELIMITER.to_string().as_str())
            .into()
    }

    fn package_specs(&self) -> Result<Vec<Spec<'_>>> {
        let specs = self
            .package
            .iter()
            .map(|s| Spec::parse(s))
            .collect::<Result<HashSet<Spec<'_>>>>()?;
        if specs.iter().any(|s| matches!(s, Spec::All)) {
            Ok(vec![Spec::All])
        } else {
            Ok(specs.into_iter().collect())
        }
    }

    fn current_package<S: PackagesSource>(&self, source: &S) -> Result<Option<S::Package>> {
        Ok(source
            .members()
            .iter()
            .find(|m| m.manifest_path() == source.runtime_manifest())
            .cloned())
    }

    fn current_selected(&self, specs: &[Spec<'_>]) -> bool {
        !self.workspace && specs.iter().any(|spec| matches!(spec, Spec::All))
    }

    fn do_match_all<S: PackagesSource>(
        specs: Vec<Spec<'_>>,
        workspace: bool,
        members: Vec<S::Package>,
    ) -> Result<Vec<S::Package>> {
        let mut packages = Vec::new();
        for spec in specs {
            packages.extend(Self::do_match::<S>(
                &spec,
                workspace,
                members.clone().into_iter(),
            )?);
        }
        packages.dedup_by_key(|p| S::package_name_of(p).to_string());
        Ok(packages)
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

#[derive(PartialEq, Eq, Hash)]
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
        let path = self.runtime_manifest.clone();
        if !path.as_str().is_empty() {
            path
        } else {
            self.workspace.manifest_path.clone()
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use camino::{Utf8Path, Utf8PathBuf};

    use crate::args::{PackagesFilter, PackagesSource, WithManifestPath};

    #[derive(Clone)]
    struct MockPackage {
        pub name: String,
        pub manifest_path: Utf8PathBuf,
    }

    impl WithManifestPath for MockPackage {
        fn manifest_path(&self) -> &Utf8Path {
            &self.manifest_path
        }
    }

    struct MockSource {
        pub members: Vec<MockPackage>,
        pub runtime_manifest: Utf8PathBuf,
    }

    impl MockSource {
        fn new(members: Vec<MockPackage>) -> Self {
            Self {
                members,
                runtime_manifest: Utf8PathBuf::from("runtime/manifest"),
            }
        }

        fn with_runtime_manifest(mut self, path: Utf8PathBuf) -> Self {
            self.runtime_manifest = path;
            self
        }
    }

    impl PackagesSource for MockSource {
        type Package = MockPackage;

        fn package_name_of(package: &Self::Package) -> &str {
            package.name.as_str()
        }

        fn members(&self) -> Vec<Self::Package> {
            self.members.clone()
        }

        fn runtime_manifest(&self) -> Utf8PathBuf {
            self.runtime_manifest.clone()
        }
    }

    fn mock_package(name: &str) -> MockPackage {
        MockPackage {
            name: name.into(),
            manifest_path: Utf8PathBuf::from(format!("package/{}", name)),
        }
    }

    fn mock_packages(names: Vec<&str>) -> Vec<MockPackage> {
        names.into_iter().map(mock_package).collect()
    }

    fn cmp_no_order(names: Vec<impl ToString>, found: Vec<impl ToString>) {
        let names: HashSet<String> = HashSet::from_iter(names.into_iter().map(|s| s.to_string()));
        let found: Vec<String> = found.into_iter().map(|p| p.to_string()).collect();
        let found: HashSet<String> = HashSet::from_iter(found);
        assert_eq!(found, names);
    }

    #[test]
    fn can_build_packages_filter() {
        let mock = MockSource::new(mock_packages(vec!["first", "second"]));

        let filter = PackagesFilter {
            package: vec!["first".into()],
            workspace: false,
        };
        let packages = filter.match_many(&mock).unwrap();
        let filter = PackagesFilter::generate_for::<MockSource>(packages.iter());
        let filter = filter.to_env();
        assert_eq!(filter, "first");

        let filter = PackagesFilter {
            package: vec!["*".into()],
            workspace: false,
        };
        let packages = filter.match_many(&mock).unwrap();
        let filter = PackagesFilter::generate_for::<MockSource>(packages.iter());
        let filter = filter.to_env();
        assert_eq!(filter, "first,second");
    }

    #[test]
    fn can_match_single_package() {
        let mock = MockSource::new(mock_packages(vec!["first", "second"]));

        let filter = PackagesFilter {
            package: vec!["second".into()],
            workspace: false,
        };
        let packages = filter.match_many(&mock).unwrap();
        assert_eq!(packages.len(), 1);
        assert_eq!(packages[0].name, "second");

        let package = filter.match_one(&mock).unwrap();
        assert_eq!(package.name, "second");
    }

    #[test]
    fn can_match_multiple_packages() {
        let names = vec!["first", "second"];
        let mock = MockSource::new(mock_packages(names.clone()));
        let filter = PackagesFilter {
            package: vec!["first".into(), "second".into()],
            workspace: false,
        };
        let packages = filter.match_many(&mock).unwrap();
        assert_eq!(packages.len(), 2);
        cmp_no_order(
            names.clone(),
            packages.into_iter().map(|p| p.name).collect(),
        );
    }

    #[test]
    fn can_match_with_glob() {
        let names = vec!["package_1", "package_2"];
        let mock = MockSource::new(mock_packages(names.clone()));
        let filter = PackagesFilter {
            package: vec!["pack*".into()],
            workspace: false,
        };
        let packages = filter.match_many(&mock).unwrap();
        assert_eq!(packages.len(), 2);
        cmp_no_order(
            names.clone(),
            packages.into_iter().map(|p| p.name).collect(),
        );
    }

    #[test]
    fn can_match_with_glob_and_package() {
        let names = vec!["package_1", "second", "package_2"];
        let mock = MockSource::new(mock_packages(names.clone()));
        let filter = PackagesFilter {
            package: vec!["pack*".into(), "second".into()],
            workspace: false,
        };
        let packages = filter.match_many(&mock).unwrap();
        assert_eq!(packages.len(), 3);
        cmp_no_order(
            names.clone(),
            packages.into_iter().map(|p| p.name).collect(),
        );
    }

    #[test]
    fn match_one_ensures_single_package() {
        let mock = MockSource::new(mock_packages(vec!["package_1", "package_2"]));
        let filter = PackagesFilter {
            package: vec!["pack*".into()],
            workspace: false,
        };
        let package = filter.match_one(&mock);
        assert!(package.is_err());
    }

    #[test]
    fn can_select_current_package() {
        let packages = mock_packages(vec!["package_1", "package_2"]);
        let mock = MockSource::new(packages.clone());
        let mock = mock.with_runtime_manifest(packages[0].manifest_path.clone());
        let filter = PackagesFilter {
            package: vec!["*".into()],
            workspace: false,
        };
        let package = filter.match_one(&mock).unwrap();
        assert_eq!(package.name, "package_1");
    }

    #[test]
    fn can_match_whole_workspace() {
        let names = vec!["package_1", "package_2"];
        let packages = mock_packages(names.clone());
        let mock = MockSource::new(packages.clone());
        let mock = mock.with_runtime_manifest(packages[0].manifest_path.clone());
        let filter = PackagesFilter {
            package: vec!["*".into()],
            workspace: true,
        };
        let packages = filter.match_many(&mock).unwrap();
        assert_eq!(packages.len(), 2);
        cmp_no_order(
            names.clone(),
            packages.into_iter().map(|p| p.name).collect(),
        );
    }
}
