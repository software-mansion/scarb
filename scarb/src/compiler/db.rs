use anyhow::{anyhow, Result};
use cairo_lang_compiler::db::{RootDatabase, RootDatabaseBuilder};
use cairo_lang_compiler::project::{AllCratesConfig, ProjectConfig, ProjectConfigContent};
use cairo_lang_defs::db::DefsGroup;
use cairo_lang_defs::ids::ModuleId;
use cairo_lang_defs::plugin::MacroPlugin;
use cairo_lang_filesystem::db::{AsFilesGroupMut, CrateSettings, FilesGroup, FilesGroupEx};
use cairo_lang_filesystem::ids::{CrateLongId, Directory};
use cairo_lang_utils::ordered_hash_map::OrderedHashMap;
use smol_str::SmolStr;
use std::sync::Arc;
use tracing::trace;

use crate::compiler::plugin::proc_macro::ProcMacroHostPlugin;
use crate::compiler::{CompilationUnit, CompilationUnitComponent};
use crate::core::Workspace;
use crate::DEFAULT_MODULE_MAIN_FILE;

// TODO(mkaput): ScarbDatabase?
pub(crate) fn build_scarb_root_database(
    unit: &CompilationUnit,
    ws: &Workspace<'_>,
) -> Result<RootDatabase> {
    let mut b = RootDatabase::builder();
    b.with_project_config(build_project_config(unit)?);
    b.with_cfg(unit.cfg_set.clone());
    load_plugins(unit, ws, &mut b)?;
    let mut db = b.build()?;
    inject_virtual_wrapper_lib(&mut db, unit)?;
    Ok(db)
}

fn load_plugins(
    unit: &CompilationUnit,
    ws: &Workspace<'_>,
    builder: &mut RootDatabaseBuilder,
) -> Result<()> {
    let mut proc_macros = ProcMacroHostPlugin::default();
    for plugin_info in &unit.cairo_plugins {
        if plugin_info.builtin {
            let package_id = plugin_info.package.id;
            let plugin = ws.config().cairo_plugins().fetch(package_id)?;
            let instance = plugin.instantiate()?;
            builder.with_plugin_suite(instance.plugin_suite());
        } else {
            proc_macros.register(plugin_info.package.clone())?;
        }
    }
    builder.with_plugin_suite(proc_macros.plugin_suite());
    Ok(())
}

/// Generates a wrapper lib file for appropriate compilation units.
///
/// This approach allows compiling crates that do not define `lib.cairo` file.
/// For example, single file crates can be created this way.
/// The actual single file module is defined as `mod` item in created lib file.
fn inject_virtual_wrapper_lib(db: &mut RootDatabase, unit: &CompilationUnit) -> Result<()> {
    let components: Vec<&CompilationUnitComponent> = unit
        .components
        .iter()
        .filter(|component| !component.package.id.is_core())
        // Skip components defining the default source path, as they already define lib.cairo files.
        .filter(|component| {
            component
                .target
                .source_path
                .file_name()
                .map(|file_name| file_name != DEFAULT_MODULE_MAIN_FILE)
                .unwrap_or(false)
        })
        .collect();

    for component in components {
        let crate_name = component.cairo_package_name();
        let crate_id = db.intern_crate(CrateLongId::Real(crate_name));
        let file_stem = component.target.source_path.file_stem().ok_or_else(|| {
            anyhow!(
                "failed to get file stem for component {}",
                component.target.source_path
            )
        })?;
        let module_id = ModuleId::CrateRoot(crate_id);
        let file_id = db.module_main_file(module_id).unwrap();
        // Inject virtual lib file wrapper.
        db.as_files_group_mut()
            .override_file_content(file_id, Some(Arc::new(format!("mod {file_stem};"))));
    }

    Ok(())
}

fn build_project_config(unit: &CompilationUnit) -> Result<ProjectConfig> {
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
            (
                component.cairo_package_name(),
                CrateSettings {
                    edition: component.package.manifest.edition,
                    cfg_set: component.cfg_set.clone(),
                    // TODO (#1040): replace this with a macro
                    experimental_features: cairo_lang_filesystem::db::ExperimentalFeaturesConfig {
                        negative_impls: experimental_features
                            .unwrap_or_default()
                            .contains(&SmolStr::new_inline("negative_impls")),
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
