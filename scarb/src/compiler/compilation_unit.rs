use std::fmt::Write;
use std::hash::{Hash, Hasher};

use cairo_lang_filesystem::cfg::CfgSet;
use smol_str::SmolStr;
use typed_builder::TypedBuilder;

use crate::compiler::Profile;
use crate::core::{ManifestCompilerConfig, Package, PackageId, Target, Workspace};
use crate::flock::Filesystem;
use scarb_stable_hash::StableHasher;

/// An object that has enough information so that Scarb knows how to build it.
#[derive(Clone, Debug)]
pub enum CompilationUnit {
    Cairo(CairoCompilationUnit),
    ProcMacro(ProcMacroCompilationUnit),
    Group(GroupCompilationUnit),
}

/// An object that has enough information so that Scarb knows how to build Cairo code with it.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct CairoCompilationUnit {
    /// The Scarb [`Package`] to be built.
    pub main_package_id: PackageId,

    /// Collection of all [`Package`]s needed to provide as _crate roots_ to
    /// the Cairo compiler in order to build `package`.
    ///
    /// ## Invariants
    ///
    /// For performance purposes, the component describing the main package is always **first**,
    /// and then it is followed by a component describing the `core` package.
    pub components: Vec<CompilationUnitComponent>,

    /// Collection of all [`Package`]s needed to load as _cairo plugins_ in order to build
    /// `package`.
    pub cairo_plugins: Vec<CompilationUnitCairoPlugin>,

    /// The profile contains information about *how* the build should be run, including debug
    /// level, etc.
    pub profile: Profile,

    /// Cairo compiler configuration parameters to use in this unit.
    pub compiler_config: ManifestCompilerConfig,

    /// Items for the Cairo's `#[cfg(...)]` attribute to be enabled in this unit.
    ///
    /// Each individual component can override this value.
    pub cfg_set: CfgSet,
}

/// A compilation unit that builds multiple Cairo modules together.
///
/// This is an internal optimization marking compilation units that *can* be built together,
/// to avoid rebuilding the same module multiple times.
/// This should not be exposed to the user or other tooling.
#[derive(Clone, Debug)]
#[non_exhaustive]
#[doc(hidden)]
pub struct GroupCompilationUnit {
    pub(crate) compilation_units: Vec<CairoCompilationUnit>,
    main_package_id: PackageId,
    components: Vec<CompilationUnitComponent>,
}

/// An object that has enough information so that Scarb knows how to build procedural macro with it.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct ProcMacroCompilationUnit {
    /// The Scarb [`Package`] to be built.
    pub main_package_id: PackageId,

    /// Collection of all [`Package`]s needed in order to build `package`.
    ///
    /// ## Invariants
    ///
    /// For performance purposes, the component describing the main package is always **first**.
    pub components: Vec<CompilationUnitComponent>,

    /// Rust compiler configuration parameters to use in this unit.
    pub compiler_config: serde_json::Value,
}

/// Information about a single package that is part of a [`CompilationUnit`].
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct CompilationUnitComponent {
    /// The Scarb [`Package`] to be built.
    pub package: Package,
    /// Information about the specific target to build, out of the possible targets in `package`.
    pub target: Target,
    /// Items for the Cairo's `#[cfg(...)]` attribute to be enabled in this component.
    pub cfg_set: Option<CfgSet>,
}

/// Information about a single package that is a compiler plugin to load for [`CompilationUnit`].
#[derive(Clone, Debug, TypedBuilder)]
#[non_exhaustive]
pub struct CompilationUnitCairoPlugin {
    /// The Scarb plugin [`Package`] to load.
    pub package: Package,
    pub builtin: bool,
}

pub trait CompilationUnitAttributes {
    fn main_package_id(&self) -> PackageId;
    fn components(&self) -> &[CompilationUnitComponent];
    fn digest(&self) -> String;

    fn main_component(&self) -> &CompilationUnitComponent {
        // NOTE: This uses the order invariant of `component` field.
        let component = &self.components()[0];
        assert_eq!(component.package.id, self.main_package_id());
        component
    }

    fn target(&self) -> &Target {
        &self.main_component().target
    }

    fn id(&self) -> String {
        format!("{}-{}", self.main_package_id().name, self.digest())
    }

    fn is_sole_for_package(&self) -> bool {
        self.main_component()
            .package
            .manifest
            .targets
            .iter()
            .filter(|t| !t.is_test())
            .count()
            >= 2
    }

    fn has_custom_name(&self) -> bool {
        self.main_component().target.kind.as_str() != self.main_package_id().name.as_str()
    }

    fn name(&self) -> String {
        let mut string = String::new();

        let main_component = self.main_component();
        if self.is_sole_for_package() || self.target().is_test() {
            write!(&mut string, "{}", main_component.target.kind).unwrap();

            if self.has_custom_name() {
                write!(&mut string, "({})", main_component.target.name).unwrap();
            }

            write!(&mut string, " ").unwrap();
        }

        write!(&mut string, "{}", self.main_package_id()).unwrap();

        string
    }
}

impl CompilationUnitAttributes for CompilationUnit {
    fn main_package_id(&self) -> PackageId {
        match self {
            Self::Cairo(unit) => unit.main_package_id(),
            Self::ProcMacro(unit) => unit.main_package_id(),
            Self::Group(unit) => unit.main_package_id(),
        }
    }
    fn components(&self) -> &[CompilationUnitComponent] {
        match self {
            Self::Cairo(unit) => unit.components(),
            Self::ProcMacro(unit) => unit.components(),
            Self::Group(unit) => unit.components(),
        }
    }
    fn digest(&self) -> String {
        match self {
            Self::Cairo(unit) => unit.digest(),
            Self::ProcMacro(unit) => unit.digest(),
            Self::Group(unit) => unit.digest(),
        }
    }
}

impl CompilationUnitAttributes for GroupCompilationUnit {
    fn main_package_id(&self) -> PackageId {
        self.main_package_id
    }

    fn components(&self) -> &[CompilationUnitComponent] {
        &self.components
    }

    fn digest(&self) -> String {
        let mut hasher = StableHasher::new();
        for unit in self.compilation_units.iter() {
            unit.digest().hash(&mut hasher);
        }
        hasher.finish_as_short_hash()
    }
}

impl CompilationUnitAttributes for CairoCompilationUnit {
    fn main_package_id(&self) -> PackageId {
        self.main_package_id
    }
    fn components(&self) -> &[CompilationUnitComponent] {
        &self.components
    }

    fn digest(&self) -> String {
        let mut hasher = StableHasher::new();
        self.main_package_id.hash(&mut hasher);
        for component in &self.components {
            component.hash(&mut hasher);
        }
        self.profile.hash(&mut hasher);
        self.compiler_config.hash(&mut hasher);
        hasher.finish_as_short_hash()
    }
}

impl CompilationUnitAttributes for ProcMacroCompilationUnit {
    fn main_package_id(&self) -> PackageId {
        self.main_package_id
    }
    fn components(&self) -> &[CompilationUnitComponent] {
        &self.components
    }

    fn digest(&self) -> String {
        let mut hasher = StableHasher::new();
        self.main_package_id.hash(&mut hasher);
        for component in &self.components {
            component.hash(&mut hasher);
        }
        hasher.finish_as_short_hash()
    }
}

impl CairoCompilationUnit {
    pub fn core_package_component(&self) -> Option<&CompilationUnitComponent> {
        // NOTE: This uses the order invariant of `component` field.
        if self.components.len() < 2 {
            None
        } else {
            let component = &self.components[1];
            assert!(component.package.id.is_core());
            Some(component)
        }
    }

    pub fn target_dir(&self, ws: &Workspace<'_>) -> Filesystem {
        ws.target_dir().child(self.profile.as_str())
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
