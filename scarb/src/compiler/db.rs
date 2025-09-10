use super::plugin::collection::PluginsForComponents;
use super::{CompilationUnitComponentId, ComponentTarget};
use crate::DEFAULT_MODULE_MAIN_FILE;
use crate::compiler::plugin::proc_macro::ProcMacroHostPlugin;
use crate::compiler::{
    CairoCompilationUnit, CompilationUnitAttributes, CompilationUnitComponent,
    CompilationUnitDependency,
};
use crate::core::{Target, Workspace};
use anyhow::{Result, anyhow};
use cairo_lang_compiler::db::RootDatabase;
use cairo_lang_compiler::project::{AllCratesConfig, ProjectConfig, ProjectConfigContent};
use cairo_lang_defs::db::{DefsGroup, defs_group_input};
use cairo_lang_defs::ids::{InlineMacroExprPluginLongId, MacroPluginLongId, ModuleId};
use cairo_lang_defs::plugin::MacroPlugin;
use cairo_lang_filesystem::db::{CrateIdentifier, CrateSettings, DependencySettings, FilesGroup};
use cairo_lang_filesystem::ids::{CrateInput, CrateLongId};
use cairo_lang_filesystem::override_file_content;
use cairo_lang_semantic::db::{SemanticGroup, semantic_group_input};
use cairo_lang_semantic::ids::AnalyzerPluginLongId;
use cairo_lang_semantic::plugin::PluginSuite;
use cairo_lang_utils::ordered_hash_map::OrderedHashMap;
use salsa::{Database, Setter};
use smol_str::SmolStr;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tracing::trace;

pub struct ScarbDatabase {
    pub db: RootDatabase,
    pub proc_macros: Vec<ProcMacroHostPlugin>,
}

/// If you change something here, make sure you also change the `build_lint_database` in `scarb/src/ops/lint.rs`.
pub(crate) fn build_scarb_root_database(
    unit: &CairoCompilationUnit,
    ws: &Workspace<'_>,
    additional_plugins: Vec<PluginSuite>,
) -> Result<ScarbDatabase> {
    let mut b = RootDatabase::builder();
    b.with_project_config(build_project_config(unit)?);
    b.with_cfg(unit.cfg_set.clone());
    b.with_inlining_strategy(unit.compiler_config.inlining_strategy.clone().into());

    let PluginsForComponents {
        mut plugins,
        proc_macros,
    } = PluginsForComponents::collect(ws, unit)?;

    append_lint_plugin(plugins.get_mut(&unit.main_component().id).unwrap());

    let main_component_suite = plugins
        .get_mut(&unit.main_component().id)
        .expect("should be able to retrieve plugins for main component");

    for additional_suite in additional_plugins.iter() {
        main_component_suite.add(additional_suite.clone());
    }

    if !unit.compiler_config.enable_gas {
        b.skip_auto_withdraw_gas();
    }
    if unit.compiler_config.panic_backtrace {
        b.with_panic_backtrace();
    }
    if unit.compiler_config.unsafe_panic {
        b.with_unsafe_panic();
    }
    let mut db = b.build()?;

    apply_plugins(&mut db, plugins);
    inject_virtual_wrapper_lib(&mut db, unit)?;

    let proc_macros = proc_macros
        .into_values()
        .flat_map(|hosts| hosts.into_iter())
        .collect();
    Ok(ScarbDatabase { db, proc_macros })
}

#[cfg(feature = "scarb-lint")]
pub(crate) fn append_lint_plugin(suite: &mut PluginSuite) {
    suite.add_analyzer_plugin::<cairo_lint::plugin::CairoLintAllow>();
}

#[cfg(not(feature = "scarb-lint"))]
pub(crate) fn append_lint_plugin(_suite: &mut PluginSuite) {}

/// Sets the plugin suites for crates related to the library components
/// according to the `plugins_for_components` mapping.
fn apply_plugins(
    db: &mut RootDatabase,
    plugins_for_components: HashMap<CompilationUnitComponentId, PluginSuite>,
) {
    for (component_id, suite) in plugins_for_components {
        let crate_input = (CrateLongId::Real {
            name: component_id.cairo_package_name(),
            discriminator: component_id.to_discriminator(),
        })
        .into_crate_input(db);
        set_override_crate_plugins_from_suite(db, crate_input, suite);
    }
}

pub fn set_override_crate_plugins_from_suite(
    db: &mut RootDatabase,
    crate_input: CrateInput,
    plugins: PluginSuite,
) {
    let mut overrides = db.macro_plugin_overrides_input().clone();
    overrides.insert(
        crate_input.clone(),
        plugins.plugins.into_iter().map(MacroPluginLongId).collect(),
    );
    defs_group_input(db)
        .set_macro_plugin_overrides(db)
        .to(Some(overrides));

    let mut overrides = db.analyzer_plugin_overrides_input().clone();
    overrides.insert(
        crate_input.clone(),
        plugins
            .analyzer_plugins
            .into_iter()
            .map(AnalyzerPluginLongId)
            .collect(),
    );
    semantic_group_input(db)
        .set_analyzer_plugin_overrides(db)
        .to(Some(overrides));

    let mut overrides = db.inline_macro_plugin_overrides_input().clone();
    overrides.insert(
        crate_input,
        Arc::new(
            plugins
                .inline_macro_plugins
                .into_iter()
                .map(|(key, value)| (key, InlineMacroExprPluginLongId(value)))
                .collect(),
        ),
    );
    defs_group_input(db)
        .set_inline_macro_plugin_overrides(db)
        .to(Some(overrides));
}

/// Generates a wrapper lib file for appropriate compilation units.
///
/// This approach allows compiling crates that do not define `lib.cairo` file.
/// For example, single file crates can be created this way.
/// The actual single file modules are defined as `mod` items in created lib file.
fn inject_virtual_wrapper_lib(db: &mut RootDatabase, unit: &CairoCompilationUnit) -> Result<()> {
    let components: Vec<&CompilationUnitComponent> = unit
        .components
        .iter()
        .filter(|component| !component.package.id.is_core())
        // Skip components defining the default source path, as they already define lib.cairo files.
        .filter(|component| {
            let is_default_source_path = |target: &Target| {
                target
                    .source_path
                    .file_name()
                    .map(|file_name| file_name != DEFAULT_MODULE_MAIN_FILE)
                    .unwrap_or(false)
            };
            match &component.targets {
                ComponentTarget::Single(target) => is_default_source_path(target),
                ComponentTarget::Ungrouped(target) => is_default_source_path(target),
                ComponentTarget::Group(_targets) => true,
            }
        })
        .collect();

    for component in components {
        let crate_id = component.crate_id(db);

        let file_stems = component
            .targets
            .targets()
            .iter()
            .map(|target| {
                target
                    .source_path
                    .file_stem()
                    .map(|file_stem| format!("mod {file_stem};"))
                    .ok_or_else(|| {
                        anyhow!(
                            "failed to get file stem for component {}",
                            target.source_path
                        )
                    })
            })
            .collect::<Result<Vec<_>>>()?;
        let content = file_stems.join("\n");
        let module_id = ModuleId::CrateRoot(crate_id);
        let file_id = db.module_main_file(module_id).unwrap();
        // Inject virtual lib file wrapper.
        override_file_content!(db, file_id, Some(Arc::from(content.as_str())));
    }

    Ok(())
}

pub(crate) fn build_project_config(unit: &CairoCompilationUnit) -> Result<ProjectConfig> {
    let crate_roots: OrderedHashMap<CrateIdentifier, PathBuf> = unit
        .components
        .iter()
        .map(|component| {
            (
                component.id.to_crate_identifier(),
                component.targets.source_root().into(),
            )
        })
        .collect();

    let crates_config = unit
        .components
        .iter()
        .map(|component| {
            let experimental_features = component.package.manifest.experimental_features.clone();
            let experimental_features = experimental_features.unwrap_or_default();

            let dependencies = component
                .dependencies
                .iter()
                .filter_map(|dependency| {
                    match dependency {
                        CompilationUnitDependency::Plugin(_) => None,
                        CompilationUnitDependency::Library(component_id) => {
                            let compilation_unit_component = unit.components.iter().find(|component| component.id == *component_id)
                                .expect("Library dependency of a component is guaranteed to exist in compilation unit components");

                            Some((
                                compilation_unit_component.cairo_package_name().to_string(),
                                DependencySettings {
                                    discriminator: compilation_unit_component.id.to_discriminator()
                                },
                            ))
                        }
                    }
                })
                .collect();

            (
                component.id.to_crate_identifier(),
                CrateSettings {
                    name: Some(component.cairo_package_name()),
                    edition: component.package.manifest.edition,
                    cfg_set: component.cfg_set.clone(),
                    version: Some(component.package.id.version.clone()),
                    dependencies,
                    // TODO (#1040): replace this with a macro
                    experimental_features: cairo_lang_filesystem::db::ExperimentalFeaturesConfig {
                        negative_impls: experimental_features
                            .contains(&SmolStr::new_inline("negative_impls")),
                        coupons: experimental_features.contains(&SmolStr::new_inline("coupons")),
                        associated_item_constraints: experimental_features
                            .contains(&SmolStr::new_static("associated_item_constraints")),
                        user_defined_inline_macros: experimental_features
                            .contains(&SmolStr::new_static("user_defined_inline_macros")),
                    },
                },
            )
        })
        .collect();
    let crates_config = AllCratesConfig {
        override_map: crates_config,
        ..Default::default()
    };

    let content = ProjectConfigContent {
        crate_roots,
        crates_config,
    };

    let project_config = ProjectConfig {
        base_path: unit.main_component().package.root().into(),
        content,
    };

    trace!(?project_config);

    Ok(project_config)
}

pub(crate) fn has_plugin(
    db: &dyn Database,
    predicate: fn(&dyn MacroPlugin) -> bool,
    component: &CompilationUnitComponent,
) -> bool {
    let crate_id = component.crate_id(db);

    db.crate_macro_plugins(crate_id)
        .iter()
        .any(|plugin_id| predicate(&*plugin_id.long(db).0))
}

pub(crate) fn is_starknet_plugin(plugin: &dyn MacroPlugin) -> bool {
    // TODO: Can this be done in less "hacky" way? TypeId is not working here, because we deal with
    // trait objects.
    format!("{plugin:?}").contains("StarknetPlugin")
}

pub(crate) fn is_executable_plugin(plugin: &dyn MacroPlugin) -> bool {
    // TODO: Can this be done in less "hacky" way? TypeId is not working here, because we deal with
    // trait objects.
    format!("{plugin:?}").contains("ExecutablePlugin")
}
