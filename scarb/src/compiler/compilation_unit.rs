use anyhow::{Result, anyhow, ensure};
use cairo_lang_filesystem::cfg::CfgSet;
use cairo_lang_filesystem::db::{CrateIdentifier, FilesGroup};
use cairo_lang_filesystem::ids::{CrateId, CrateLongId};
use camino::{Utf8Path, Utf8PathBuf};
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use smol_str::SmolStr;
use std::fmt::Write;
use std::hash::{Hash, Hasher};
use std::sync::{Arc, OnceLock};
use typed_builder::TypedBuilder;

use crate::compiler::Profile;
use crate::compiler::plugin::proc_macro::ProcMacroInstance;
use crate::core::{
    ManifestCompilerConfig, Package, PackageId, PackageName, Target, TargetKind, Workspace,
};
use crate::flock::Filesystem;
use crate::{FINGERPRINT_DIR_NAME, INCREMENTAL_DIR_NAME};
use scarb_stable_hash::StableHasher;

/// An object that has enough information so that Scarb knows how to build it.
#[derive(Clone, Debug)]
pub enum CompilationUnit {
    Cairo(CairoCompilationUnit),
    ProcMacro(ProcMacroCompilationUnit),
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

    /// Instance of the proc macro loaded from prebuilt library, if available.
    pub prebuilt: Option<Arc<ProcMacroInstance>>,
}

/// Information about a single package that is part of a [`CompilationUnit`].
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct CompilationUnitComponent {
    /// Unique id identifying this component in its compilation unit.
    pub id: CompilationUnitComponentId,
    /// The Scarb [`Package`] to be built.
    pub package: Package,
    /// Information about the specific targets to build, out of the possible targets in `package`.
    ///
    /// This can either be a single target, or a group of targets that share the same kind and params.
    ///
    /// The latter acts as an optimization that allows building multiple source files (identified
    /// by target source paths) in a single compilation unit (e.g. separate tests files from `tests/`
    /// directory in the same package).
    ///
    /// This translates to `group-id` field in the toml manifest of the project.
    pub targets: ComponentTarget,
    /// Items for the Cairo's `#[cfg(...)]` attribute to be enabled in this component.
    pub cfg_set: Option<CfgSet>,
    /// Dependencies of this component.
    /// Contains libraries and plugins, represented uniquely in the scope of the compilation unit.
    pub dependencies: Vec<CompilationUnitDependency>,
}

impl CompilationUnitComponent {
    /// Returns a [`CrateId`] of a crate associated with the [`CompilationUnitComponent`].
    pub fn crate_id<'db>(&self, db: &'db dyn FilesGroup) -> CrateId<'db> {
        db.intern_crate(CrateLongId::Real {
            name: self.cairo_package_name(),
            discriminator: self.id.to_discriminator(),
        })
    }
}

impl From<&CompilationUnitComponent>
    for scarb_proc_macro_server_types::scope::CompilationUnitComponent
{
    fn from(value: &CompilationUnitComponent) -> Self {
        // `name` and `discriminator` used here must be identital to the scarb-metadata.
        // This implementation detail is crucial for communication between PMS and LS.
        // Always remember to verify this invariant when changing the internals.
        Self {
            name: value.cairo_package_name().to_string(),
            discriminator: value.id.to_discriminator().map(Into::into),
        }
    }
}

/// The kind of the compilation unit dependency.
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub enum CompilationUnitDependency {
    Library(CompilationUnitComponentId),
    Plugin(CompilationUnitComponentId),
}

impl CompilationUnitDependency {
    pub fn component_id(&self) -> &CompilationUnitComponentId {
        match self {
            CompilationUnitDependency::Library(id) => id,
            CompilationUnitDependency::Plugin(id) => id,
        }
    }
}

/// Information about a single package that is a compiler plugin to load for [`CompilationUnit`].
#[derive(Clone, Debug, TypedBuilder)]
#[non_exhaustive]
pub struct CompilationUnitCairoPlugin {
    /// An id which uniquely identifies the plugin in scope of the compilation unit
    /// amongst other plugins and [`CompilationUnitComponent`]s.
    /// It is used to identify the plugin as a possible dependency of a [`CompilationUnitComponent`].
    pub component_dependency_id: CompilationUnitComponentId,
    /// The Scarb plugin [`Package`] to load.
    pub package: Package,
    /// Indicate whether the plugin is built into Scarb, or compiled from source.
    pub builtin: bool,
    /// Indicate whether a prebuilt plugin binary should be attempted to be loaded.
    pub prebuilt_allowed: bool,
    /// Instance of the proc macro loaded from prebuilt library, if available.
    pub prebuilt: Option<Arc<ProcMacroInstance>>,
    /// Location of the shared library for the package.
    pub cached_shared_lib_path: Arc<OnceLock<Utf8PathBuf>>,
}

/// Unique identifier of the compilation unit component.
/// Currently, a compilation unit can be uniquely identified by [`PackageId`] only.
/// It may be not sufficient in the future depending on changes to the compilation model.
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct CompilationUnitComponentId {
    pub package_id: PackageId,
}

impl CompilationUnitComponentId {
    /// Returns a name of the corresponding package.
    pub fn cairo_package_name(&self) -> String {
        self.package_id.name.to_string()
    }
}

impl CompilationUnitComponentId {
    pub fn to_metadata_component_id(&self) -> scarb_metadata::CompilationUnitComponentId {
        self.package_id.to_serialized_string().into()
    }

    pub fn to_discriminator(&self) -> Option<String> {
        if self.package_id.name == PackageName::CORE {
            None
        } else {
            Some(self.to_crate_identifier().into())
        }
    }

    pub fn to_crate_identifier(&self) -> CrateIdentifier {
        self.package_id.to_serialized_string().into()
    }
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
        self.main_component().target_kind().as_str() != self.main_package_id().name.as_str()
    }

    fn name(&self) -> String {
        let mut string = String::new();

        let main_component = self.main_component();
        if self.is_sole_for_package() || self.main_component().target_kind().is_test() {
            write!(&mut string, "{}", main_component.target_kind()).unwrap();

            if self.has_custom_name() {
                write!(&mut string, "({})", main_component.target_name()).unwrap();
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
        }
    }
    fn components(&self) -> &[CompilationUnitComponent] {
        match self {
            Self::Cairo(unit) => unit.components(),
            Self::ProcMacro(unit) => unit.components(),
        }
    }
    fn digest(&self) -> String {
        match self {
            Self::Cairo(unit) => unit.digest(),
            Self::ProcMacro(unit) => unit.digest(),
        }
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

    pub fn fingerprint_dir(&self, ws: &Workspace<'_>) -> Filesystem {
        self.target_dir(ws).child(FINGERPRINT_DIR_NAME)
    }

    pub fn incremental_cache_dir(&self, ws: &Workspace<'_>) -> Filesystem {
        self.target_dir(ws).child(INCREMENTAL_DIR_NAME)
    }

    /// Rewrite single compilation unit with multiple targets, into multiple compilation units
    /// with single targets.
    pub fn rewrite_to_single_source_paths(&self) -> Vec<Self> {
        let rewritten_main = self
            .main_component()
            .targets
            .targets()
            .iter()
            .map(|target| {
                let mut main = self.main_component().clone();
                main.targets = ComponentTarget::new_ungrouped(target.clone());
                main
            })
            .collect_vec();

        let mut components = self.components.clone();
        components.remove(0);

        rewritten_main
            .into_iter()
            .map(|component| {
                let mut unit = self.clone();
                unit.components = vec![component];
                unit.components.extend(components.clone());
                unit
            })
            .collect_vec()
    }
}

impl CompilationUnitComponent {
    /// Validate input and create new [CompilationUnitComponent] instance.
    pub fn try_new(
        package: Package,
        targets: ComponentTarget,
        cfg_set: Option<CfgSet>,
    ) -> Result<Self> {
        Ok(Self {
            id: CompilationUnitComponentId {
                package_id: package.id,
            },
            package,
            targets,
            cfg_set,
            dependencies: vec![],
        })
    }

    pub fn target_kind(&self) -> TargetKind {
        self.targets.target_kind()
    }

    pub fn target_name(&self) -> SmolStr {
        self.targets.target_name()
    }

    pub fn cairo_package_name(&self) -> String {
        self.package.id.name.to_string()
    }

    fn hash(&self, hasher: &mut impl Hasher) {
        self.package.id.hash(hasher);
        self.targets.hash(hasher);
    }
}

#[derive(Clone, Debug, Hash)]
pub enum ComponentTarget {
    Single(Target),
    Group(Vec<Target>),
    Ungrouped(Target),
}

impl ComponentTarget {
    pub fn try_new_group(targets: Vec<Target>) -> Result<Self> {
        ensure!(
            !targets.is_empty(),
            "a compilation unit component must have at least one target"
        );
        if targets.len() == 1 {
            return Ok(Self::Single(targets.into_iter().next().unwrap()));
        }
        ensure!(
            targets
                .iter()
                .map(|t| &t.kind)
                .collect::<std::collections::HashSet<_>>()
                .len()
                == 1,
            "all targets in a compilation unit component must have the same kind"
        );
        ensure!(
            targets
                .iter()
                .map(|t| &t.params)
                .all(|p| *p == targets[0].params),
            "all targets in a compilation unit component must have the same params"
        );
        ensure!(
            targets
                .iter()
                .map(|t| t.source_root())
                .all(|p| p == targets[0].source_root()),
            "all targets in a compilation unit component must have the same source path parent"
        );
        ensure!(
            targets.iter().all(|t| t.group_id.is_some()),
            "all targets in a compilation unit component with multiple targets must have group_id defined"
        );
        Ok(Self::Group(targets))
    }

    pub fn new_single(target: Target) -> Self {
        Self::Single(target)
    }

    pub fn new_ungrouped(target: Target) -> Self {
        Self::Ungrouped(target)
    }

    pub fn targets(&self) -> &[Target] {
        match self {
            Self::Single(target) => std::slice::from_ref(target),
            Self::Ungrouped(target) => std::slice::from_ref(target),
            Self::Group(targets) => targets,
        }
    }

    pub fn extract_single(&self) -> Result<&Target> {
        match self {
            Self::Single(target) => Ok(target),
            Self::Ungrouped(target) => Ok(target),
            Self::Group(_) => Err(anyhow!("expected a single target, found target group")),
        }
    }

    pub fn source_root(&self) -> &Utf8Path {
        self.targets()[0].source_root()
    }

    pub fn target_name(&self) -> SmolStr {
        let grouped_target_name = |target: &Target| {
            target
                .group_id
                .clone()
                .unwrap_or_else(|| target.name.clone())
        };
        match self {
            Self::Ungrouped(target) => target.name.clone(),
            Self::Single(target) => grouped_target_name(target),
            Self::Group(targets) => grouped_target_name(&targets[0]),
        }
    }

    pub fn target_kind(&self) -> TargetKind {
        self.targets()[0].kind.clone()
    }

    pub fn target_props<'de, P>(&self) -> Result<P>
    where
        P: Default + Serialize + Deserialize<'de>,
    {
        self.targets()[0].props::<P>()
    }

    pub fn source_paths(&self) -> Vec<&Utf8Path> {
        self.targets()
            .iter()
            .map(|t| t.source_path.as_path())
            .collect()
    }
}
