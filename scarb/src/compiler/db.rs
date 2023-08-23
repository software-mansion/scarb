use std::sync::Arc;

use anyhow::Result;
use cairo_lang_compiler::db::RootDatabase;
use cairo_lang_compiler::project::{ProjectConfig, ProjectConfigContent};
use cairo_lang_defs::db::DefsGroup;
use cairo_lang_defs::plugin::MacroPlugin;
use cairo_lang_filesystem::ids::Directory;
use cairo_lang_starknet::inline_macros::selector::SelectorMacro;
use tracing::trace;

use crate::compiler::CompilationUnit;
use crate::core::Workspace;

// TODO(mkaput): ScarbDatabase?
pub(crate) fn build_scarb_root_database(
    unit: &CompilationUnit,
    ws: &Workspace<'_>,
) -> Result<RootDatabase> {
    let mut b = RootDatabase::builder();
    b.with_project_config(build_project_config(unit)?);
    b.with_cfg(unit.cfg_set.clone());

    for plugin_info in &unit.cairo_plugins {
        let package_id = plugin_info.package.id;
        let plugin = ws.config().cairo_plugins().fetch(package_id)?;
        let instance = plugin.instantiate()?;
        for macro_plugin in instance.macro_plugins() {
            // HACK(mkaput): Remove this when #575 will be done.
            if is_starknet_plugin(&*macro_plugin) {
                b.with_inline_macro_plugin(SelectorMacro::NAME, Arc::new(SelectorMacro));
            }

            b.with_macro_plugin(macro_plugin);
        }
    }

    b.build()
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

    let corelib = Some(Directory::Real(
        unit.core_package_component().target.source_root().into(),
    ));

    let content = ProjectConfigContent { crate_roots };

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
