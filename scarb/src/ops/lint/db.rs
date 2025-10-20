use crate::{
    DEFAULT_MODULE_MAIN_FILE,
    compiler::{
        CairoCompilationUnit, CompilationUnitAttributes, CompilationUnitComponent,
        CompilationUnitComponentId, ComponentTarget,
        db::{append_lint_plugin, build_project_config},
        plugin::collection::PluginsForComponents,
    },
    core::{Target, Workspace},
};
use anyhow::Result;
use anyhow::anyhow;
use cairo_lang_defs::{
    db::{DefsGroup, defs_group_input},
    ids::{InlineMacroExprPluginLongId, MacroPluginLongId, ModuleId},
};
use cairo_lang_filesystem::{
    db::FilesGroup,
    ids::{CrateInput, CrateLongId, SmolStrId},
    override_file_content,
};
use cairo_lang_semantic::{
    db::{SemanticGroup, semantic_group_input},
    ids::AnalyzerPluginLongId,
    plugin::PluginSuite,
};
use cairo_lint::LinterAnalysisDatabase;
use salsa::Setter;
use std::{collections::HashMap, sync::Arc};

/// Keep it in sync with [crate::compiler::db::build_scarb_root_database].
pub fn build_lint_database(
    unit: &CairoCompilationUnit,
    ws: &Workspace<'_>,
) -> Result<LinterAnalysisDatabase> {
    let mut b = LinterAnalysisDatabase::builder();
    b.with_project_config(build_project_config(unit)?);
    b.with_cfg(unit.cfg_set.clone());

    let PluginsForComponents { mut plugins, .. } = PluginsForComponents::collect(ws, unit)?;

    append_lint_plugin(plugins.get_mut(&unit.main_component().id).unwrap());

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

    Ok(db)
}

/// Sets the plugin suites for crates related to the library components
/// according to the `plugins_for_components` mapping.
fn apply_plugins(
    db: &mut LinterAnalysisDatabase,
    plugins_for_components: HashMap<CompilationUnitComponentId, PluginSuite>,
) {
    for (component_id, suite) in plugins_for_components {
        let crate_input = CrateLongId::Real {
            name: SmolStrId::from(db, component_id.cairo_package_name()),
            discriminator: component_id.to_discriminator(),
        }
        .into_crate_input(db);
        set_override_crate_plugins_from_suite(db, crate_input, suite);
    }
}

/// Generates a wrapper lib file for appropriate compilation units.
///
/// This approach allows compiling crates that do not define `lib.cairo` file.
/// For example, single file crates can be created this way.
/// The actual single file modules are defined as `mod` items in created lib file.
fn inject_virtual_wrapper_lib(
    db: &mut LinterAnalysisDatabase,
    unit: &CairoCompilationUnit,
) -> Result<()> {
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

fn set_override_crate_plugins_from_suite(
    db: &mut LinterAnalysisDatabase,
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
