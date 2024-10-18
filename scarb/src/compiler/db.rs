use anyhow::{anyhow, Result};
use cairo_lang_compiler::db::{RootDatabase, RootDatabaseBuilder};
use cairo_lang_compiler::project::{AllCratesConfig, ProjectConfig, ProjectConfigContent};
use cairo_lang_defs::db::DefsGroup;
use cairo_lang_defs::ids::ModuleId;
use cairo_lang_defs::plugin::MacroPlugin;
use cairo_lang_filesystem::db::{
    AsFilesGroupMut, CrateSettings, DependencySettings, FilesGroup, FilesGroupEx,
    CORELIB_CRATE_NAME,
};
use cairo_lang_filesystem::ids::{CrateLongId, Directory};
use cairo_lang_utils::ordered_hash_map::OrderedHashMap;
use smol_str::{SmolStr, ToSmolStr};
use std::collections::BTreeMap;
use std::sync::Arc;
use tracing::trace;

use crate::compiler::plugin::proc_macro::{ProcMacroHost, ProcMacroHostPlugin};
use crate::compiler::{CairoCompilationUnit, CompilationUnitAttributes, CompilationUnitComponent};
use crate::core::{ManifestDependency, TestTargetProps, TestTargetType, Workspace};
use crate::DEFAULT_MODULE_MAIN_FILE;

pub struct ScarbDatabase {
    pub db: RootDatabase,
    pub proc_macro_host: Arc<ProcMacroHostPlugin>,
}

pub(crate) fn build_scarb_root_database(
    unit: &CairoCompilationUnit,
    ws: &Workspace<'_>,
) -> Result<ScarbDatabase> {
    let mut b = RootDatabase::builder();
    b.with_project_config(build_project_config(unit)?);
    b.with_cfg(unit.cfg_set.clone());
    b.with_inlining_strategy(unit.compiler_config.inlining_strategy.clone().into());
    let proc_macro_host = load_plugins(unit, ws, &mut b)?;
    if !unit.compiler_config.enable_gas {
        b.skip_auto_withdraw_gas();
    }
    if unit.compiler_config.add_redeposit_gas {
        b.with_add_redeposit_gas();
    }
    let mut db = b.build()?;
    inject_virtual_wrapper_lib(&mut db, unit)?;
    Ok(ScarbDatabase {
        db,
        proc_macro_host,
    })
}

fn load_plugins(
    unit: &CairoCompilationUnit,
    ws: &Workspace<'_>,
    builder: &mut RootDatabaseBuilder,
) -> Result<Arc<ProcMacroHostPlugin>> {
    let mut proc_macros = ProcMacroHost::default();
    for plugin_info in &unit.cairo_plugins {
        if plugin_info.builtin {
            let package_id = plugin_info.package.id;
            let plugin = ws.config().cairo_plugins().fetch(package_id)?;
            let instance = plugin.instantiate()?;
            builder.with_plugin_suite(instance.plugin_suite());
        } else {
            proc_macros.register(plugin_info.package.clone(), ws.config())?;
        }
    }
    let macro_host = Arc::new(proc_macros.into_plugin()?);
    builder.with_plugin_suite(ProcMacroHostPlugin::build_plugin_suite(macro_host.clone()));
    Ok(macro_host)
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
            !component.targets.is_empty()
                && (component.targets.len() > 1
                    || component
                        .first_target()
                        .source_path
                        .file_name()
                        .map(|file_name| file_name != DEFAULT_MODULE_MAIN_FILE)
                        .unwrap_or(false))
        })
        .collect();

    for component in components {
        let name = component.cairo_package_name();
        let version = component.package.id.version.clone();
        let crate_id = db.intern_crate(CrateLongId::Real {
            name,
            discriminator: Some(version).map(|v| v.to_smolstr()),
        });
        let file_stems = component
            .targets
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
        db.as_files_group_mut()
            .override_file_content(file_id, Some(Arc::from(content.as_str())));
    }

    Ok(())
}

fn build_project_config(unit: &CairoCompilationUnit) -> Result<ProjectConfig> {
    let crate_roots = unit
        .components
        .iter()
        .filter(|component| !component.package.id.is_core())
        .map(|component| {
            (
                component.cairo_package_name(),
                component.first_target().source_root().into(),
            )
        })
        .collect();

    let crates_config: OrderedHashMap<SmolStr, CrateSettings> = unit
        .components
        .iter()
        .map(|component| {
            let experimental_features = component.package.manifest.experimental_features.clone();
            let experimental_features = experimental_features.unwrap_or_default();
            // Those are direct dependencies of the component.
            let dependencies_summary: Vec<&ManifestDependency> = component
                .package
                .manifest
                .summary
                .full_dependencies()
                .collect();

            // We iterate over all of the compilation unit components to get dependency's version.
            let mut dependencies: BTreeMap<String, DependencySettings> = unit
                .components
                .iter()
                .filter(|component_as_dependency| {
                    dependencies_summary.iter().any(|dependency_summary| {
                        dependency_summary.name == component_as_dependency.package.id.name
                    }) ||
                        // This is a hacky way of accommodating integration test components,
                        // which need to depend on the tested package.
                        component_as_dependency
                        .package
                        .manifest
                        .targets
                        .iter()
                        .filter(|target| target.kind.is_test())
                        .any(|target| {
                            target.group_id.clone().unwrap_or(target.name.clone())
                                == component.package.id.name.to_smol_str()
                            && component_as_dependency.cairo_package_name() != component.cairo_package_name()
                        })
                })
                .map(|compilation_unit_component| {
                    (
                        compilation_unit_component.package.id.name.to_string(),
                        DependencySettings {
                            discriminator: (compilation_unit_component.package.id.name.to_string()
                                != *CORELIB_CRATE_NAME)
                                .then_some(compilation_unit_component.package.id.version.clone())
                                .map(|v| v.to_smolstr()),
                        },
                    )
                })
                .collect();

            // Adds itself to dependencies
            let is_integration_test = if component.first_target().kind.is_test() {
                let props: Option<TestTargetProps> = component.first_target().props().ok();
                props
                    .map(|props| props.test_type == TestTargetType::Integration)
                    .unwrap_or_default()
            } else { false };
            if !is_integration_test {
                dependencies.insert(
                    component.package.id.name.to_string(),
                    DependencySettings {
                        discriminator: (component.package.id.name.to_string() != *CORELIB_CRATE_NAME)
                            .then_some(component.package.id.version.clone()).map(|v| v.to_smolstr()),
                    },
                );
            }

            (
                component.cairo_package_name(),
                CrateSettings {
                    edition: component.package.manifest.edition,
                    cfg_set: component.cfg_set.clone(),
                    version: Some(component.package.id.version.clone()),
                    dependencies,
                    // TODO (#1040): replace this with a macro
                    experimental_features: cairo_lang_filesystem::db::ExperimentalFeaturesConfig {
                        negative_impls: experimental_features
                            .contains(&SmolStr::new_inline("negative_impls")),
                        coupons: experimental_features.contains(&SmolStr::new_inline("coupons")),
                    },
                },
            )
        })
        .collect();
    let crates_config = AllCratesConfig {
        override_map: crates_config,
        ..Default::default()
    };

    let corelib = unit
        .core_package_component()
        .map(|core| Directory::Real(core.first_target().source_root().into()));

    let content = ProjectConfigContent {
        crate_roots,
        crates_config,
    };

    let project_config = ProjectConfig {
        base_path: unit.main_component().package.root().into(),
        corelib,
        content,
    };

    trace!(?project_config);

    Ok(project_config)
}

pub(crate) fn has_starknet_plugin(db: &RootDatabase) -> bool {
    db.macro_plugins()
        .iter()
        .any(|plugin| is_starknet_plugin(&**plugin))
}

fn is_starknet_plugin(plugin: &dyn MacroPlugin) -> bool {
    // Can this be done in less "hacky" way? TypeId is not working here, because we deal with
    // trait objects.
    format!("{:?}", plugin).contains("StarkNetPlugin")
}
