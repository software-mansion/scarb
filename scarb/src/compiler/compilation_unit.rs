use std::fmt::Write;
use std::hash::Hash;

use crate::compiler::Profile;
use crate::core::manifest::ManifestCompilerConfig;
use crate::core::{Package, Target};
use crate::internal::stable_hash::StableHasher;

/// An object that has enough information so that Scarb knows how to build it.
#[derive(Clone, Debug)]
pub struct CompilationUnit {
    /// The Scarb [`Package`] to be build.
    pub package: Package,
    /// Information about the specific target to build, out of the possible targets in `package`.
    pub target: Target,
    /// Collection of all [`Package`]s needed to provide as _crate roots_ to
    /// the Cairo compiler in order to build `package`.
    pub components: Vec<Package>,
    /// The profile contains information about *how* the build should be run, including debug
    /// level, etc.
    pub profile: Profile,
    /// Cairo compiler configuration parameters to use in this unit.
    pub compiler_config: ManifestCompilerConfig,
}

impl CompilationUnit {
    pub fn is_sole_for_package(&self) -> bool {
        self.package.manifest.targets.len() >= 2
    }

    pub fn has_custom_name(&self) -> bool {
        self.target.kind != self.package.id.name.as_str()
    }

    pub fn id(&self) -> String {
        format!("{}-{}", self.package.id.name, self.digest())
    }

    pub fn name(&self) -> String {
        let mut string = String::new();

        if self.is_sole_for_package() {
            write!(&mut string, "{}", self.target.kind).unwrap();

            if self.has_custom_name() {
                write!(&mut string, "({})", self.target.name).unwrap();
            }

            write!(&mut string, " ").unwrap();
        }

        write!(&mut string, "{}", self.package.id).unwrap();

        string
    }

    pub fn digest(&self) -> String {
        let mut hasher = StableHasher::new();
        self.package.id.hash(&mut hasher);
        self.target.hash(&mut hasher);
        for component in &self.components {
            component.id.hash(&mut hasher);
        }
        self.profile.name.hash(&mut hasher);
        self.compiler_config.hash(&mut hasher);
        hasher.finish_as_short_hash()
    }
}
