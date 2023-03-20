use std::fmt::Write;
use std::hash::{Hash, Hasher};

use smol_str::SmolStr;

use crate::compiler::Profile;
use crate::core::manifest::ManifestCompilerConfig;
use crate::core::{Package, PackageId, Target};
use crate::internal::stable_hash::StableHasher;

/// An object that has enough information so that Scarb knows how to build it.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct CompilationUnit {
    /// The Scarb [`Package`] to be build.
    pub main_package_id: PackageId,
    /// Collection of all [`Package`]s needed to provide as _crate roots_ to
    /// the Cairo compiler in order to build `package`.
    ///
    /// ## Invariants
    ///
    /// For performance purposes, the component describing the main package is always **first**,
    /// and then it is followed by a component describing the `core` package.
    pub components: Vec<CompilationUnitComponent>,
    /// The profile contains information about *how* the build should be run, including debug
    /// level, etc.
    pub profile: Profile,
    /// Cairo compiler configuration parameters to use in this unit.
    pub compiler_config: ManifestCompilerConfig,
}

/// Information about a single package that is part of a [`CompilationUnit`].
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct CompilationUnitComponent {
    /// The Scarb [`Package`] to be build.
    pub package: Package,
    /// Information about the specific target to build, out of the possible targets in `package`.
    pub target: Target,
}

impl CompilationUnit {
    pub fn main_component(&self) -> &CompilationUnitComponent {
        // NOTE: This uses the order invariant of `component` field.
        let component = &self.components[0];
        assert_eq!(component.package.id, self.main_package_id);
        component
    }

    pub fn core_package_component(&self) -> &CompilationUnitComponent {
        // NOTE: This uses the order invariant of `component` field.
        let component = &self.components[1];
        assert!(component.package.id.is_core());
        component
    }

    pub fn target(&self) -> &Target {
        &self.main_component().target
    }

    pub fn is_sole_for_package(&self) -> bool {
        self.main_component().package.manifest.targets.len() >= 2
    }

    pub fn has_custom_name(&self) -> bool {
        self.main_component().target.kind != self.main_package_id.name.as_str()
    }

    pub fn id(&self) -> String {
        format!("{}-{}", self.main_package_id.name, self.digest())
    }

    pub fn name(&self) -> String {
        let mut string = String::new();

        let main_component = self.main_component();
        if self.is_sole_for_package() {
            write!(&mut string, "{}", main_component.target.kind).unwrap();

            if self.has_custom_name() {
                write!(&mut string, "({})", main_component.target.name).unwrap();
            }

            write!(&mut string, " ").unwrap();
        }

        write!(&mut string, "{}", self.main_package_id).unwrap();

        string
    }

    pub fn digest(&self) -> String {
        let mut hasher = StableHasher::new();
        self.main_package_id.hash(&mut hasher);
        for component in &self.components {
            component.hash(&mut hasher);
        }
        self.profile.name.hash(&mut hasher);
        self.compiler_config.hash(&mut hasher);
        hasher.finish_as_short_hash()
    }
}

impl CompilationUnitComponent {
    pub fn cairo_package_name(&self) -> SmolStr {
        self.package.id.name.to_smol_str()
    }

    fn hash(&self, hasher: &mut impl Hasher) {
        self.package.id.hash(hasher);
        self.target.hash(hasher);
    }
}
