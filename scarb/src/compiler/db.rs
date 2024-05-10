use anyhow::Result;
use cairo_lang_compiler::db::{RootDatabase, RootDatabaseBuilder};
use cairo_lang_compiler::project::{AllCratesConfig, ProjectConfig, ProjectConfigContent};
use cairo_lang_defs::db::DefsGroup;
use cairo_lang_defs::ids::ModuleId;
use cairo_lang_defs::plugin::MacroPlugin;
use cairo_lang_filesystem::db::{AsFilesGroupMut, CrateSettings, FilesGroup, FilesGroupEx};
use cairo_lang_filesystem::ids::{CrateLongId, Directory};
use cairo_lang_utils::ordered_hash_map::OrderedHashMap;
use itertools::Itertools;
use smol_str::SmolStr;
use std::sync::Arc;
use tracing::trace;

use crate::compiler::plugin::proc_macro::{ProcMacroHost, ProcMacroHostPlugin};
use crate::compiler::{CairoCompilationUnit, CompilationUnitAttributes, GroupCompilationUnit};
use crate::core::Workspace;

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
    let proc_macro_host = load_plugins(unit, ws, &mut b)?;
    if !unit.compiler_config.enable_gas {
        b.skip_auto_withdraw_gas();
    }
    Ok(ScarbDatabase {
        db: b.build()?,
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

/// Generates a wrapper lib file for group compilation units.
///
/// This approach allows compiling multiple creates together as a single module.
/// The actual single file module is defined as `mod` item in created lib file.
pub(crate) fn inject_virtual_wrapper_lib_for_group(
    db: &mut RootDatabase,
    group: &GroupCompilationUnit,
) -> Result<()> {
    let component = group.main_component();
    let crate_name = component.cairo_package_name();
    let crate_id = db.intern_crate(CrateLongId::Real(crate_name));
    let module_id = ModuleId::CrateRoot(crate_id);
    let file_id = db.module_main_file(module_id).unwrap();
    // Inject virtual lib file wrapper.
    let group = group
        .compilation_units
        .iter()
        .filter_map(|comp| comp.target().source_path.file_name())
        .map(|comp| format!("mod {comp};"))
        .collect_vec();
    let content = group.join("\n");
    db.as_files_group_mut()
        .override_file_content(file_id, Some(Arc::new(content)));
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
                component.target.source_root().into(),
            )
        })
        .collect();

    let crates_config: OrderedHashMap<SmolStr, CrateSettings> = unit
        .components
        .iter()
        .map(|component| {
            let experimental_features = component.package.manifest.experimental_features.clone();
            let experimental_features = experimental_features.unwrap_or_default();
            (
                component.cairo_package_name(),
                CrateSettings {
                    edition: component.package.manifest.edition,
                    cfg_set: component.cfg_set.clone(),
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
        .map(|core| Directory::Real(core.target.source_root().into()));

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
