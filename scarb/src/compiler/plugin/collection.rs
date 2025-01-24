use std::{collections::HashMap, sync::Arc};

use anyhow::Context;
use cairo_lang_semantic::{inline_macros::get_default_plugin_suite, plugin::PluginSuite};

use crate::{
    compiler::{CairoCompilationUnit, CompilationUnitComponentId, CompilationUnitDependency},
    core::Workspace,
};

use super::proc_macro::{ProcMacroHostPlugin, ProcMacroRepository};

pub struct PluginsForComponents {
    pub plugins: HashMap<CompilationUnitComponentId, PluginSuite>,
    pub proc_macros: HashMap<CompilationUnitComponentId, Arc<ProcMacroHostPlugin>>,
}

impl PluginsForComponents {
    pub fn collect(workspace: &Workspace<'_>, unit: &CairoCompilationUnit) -> anyhow::Result<Self> {
        let mut plugins = collect_builtin_plugins(workspace, unit)?;
        let proc_macros = collect_proc_macros(workspace, unit)?;

        for (component_id, suite) in plugins.iter_mut() {
            if let Some(proc_macro) = proc_macros.get(component_id) {
                suite.add(ProcMacroHostPlugin::build_plugin_suite(proc_macro.clone()));
            }
        }

        Ok(Self {
            plugins,
            proc_macros,
        })
    }
}

/// A container for Proc Macro Server to manage macros present in the analyzed workspace.
pub struct WorkspaceProcMacros {
    /// A mapping of the form: PackageId (CU Member) -> CompilationUnitComponentId -> plugin, in a serialized form.
    pub macros_for_compilation_units: HashMap<String, HashMap<String, Arc<ProcMacroHostPlugin>>>,
}

impl WorkspaceProcMacros {
    /// Collects and groups procedural macros for all the components of all the compilation unit in the workspace.
    pub fn collect(
        workspace: &Workspace<'_>,
        compilation_units: &[&CairoCompilationUnit],
    ) -> anyhow::Result<Self> {
        let macros_for_compilation_units = compilation_units
            .iter()
            .map(|unit| {
                let unit_id = unit.main_package_id.to_serialized_string();
                let plugins = collect_proc_macros(workspace, unit)?
                    .into_iter()
                    .map(|(component_id, component_plugins)| {
                        (
                            component_id
                                .to_discriminator()
                                .unwrap_or_else(|| component_id.to_crate_identifier().into()) // Relevant only for corelib
                                .to_string(),
                            component_plugins,
                        )
                    })
                    .collect();
                Ok((unit_id, plugins))
            })
            .collect::<anyhow::Result<HashMap<_, _>>>()?;

        Ok(Self {
            macros_for_compilation_units,
        })
    }

    /// Returns a `ProcMacroHostPlugin` assigned to the component with `cu_component_id`
    /// in a compilation unit which member package has `cu_member_id`.
    pub fn get(
        &self,
        compilation_unit_member_id: &str,
        compilation_unit_component_id: &str,
    ) -> anyhow::Result<Arc<ProcMacroHostPlugin>> {
        self.macros_for_compilation_units
        .get(compilation_unit_member_id)
        .with_context(|| format!("Compilation unit `{compilation_unit_member_id}` not found"))?
        .get(compilation_unit_component_id)
        .with_context(|| format!("Component `{compilation_unit_component_id}` of the compilation unit `{compilation_unit_member_id}` not found"))
        .cloned()
    }
}

/// Builds `PluginSuite`s for each component of the [`CairoCompilationUnit`],
/// according to the dependencies on builtin macros.
fn collect_builtin_plugins(
    workspace: &Workspace<'_>,
    unit: &CairoCompilationUnit,
) -> anyhow::Result<HashMap<CompilationUnitComponentId, PluginSuite>> {
    let mut plugin_suites = HashMap::new();

    for component in unit.components.iter() {
        let mut component_suite = get_default_plugin_suite();

        for dependency in component.dependencies.iter() {
            if matches!(dependency, CompilationUnitDependency::Library(_)) {
                continue;
            }

            let Some(plugin) = unit
                .cairo_plugins
                .iter()
                .find(|plugin| &plugin.component_dependency_id == dependency.component_id())
            else {
                continue;
            };

            if !plugin.builtin {
                continue;
            }

            let package_id = plugin.package.id;
            let plugin = workspace.config().cairo_plugins().fetch(package_id)?;
            let instance = plugin.instantiate()?;
            let suite = instance.plugin_suite();
            component_suite.add(suite);
        }

        plugin_suites.insert(component.id.clone(), component_suite);
    }

    Ok(plugin_suites)
}

/// Builds [`ProcMacroHostPlugin`]s for each component of the [`CairoCompilationUnit`],
/// according to the dependencies on procedural macros.
fn collect_proc_macros(
    workspace: &Workspace<'_>,
    unit: &CairoCompilationUnit,
) -> anyhow::Result<HashMap<CompilationUnitComponentId, Arc<ProcMacroHostPlugin>>> {
    let mut proc_macro_repository = ProcMacroRepository::default();
    let mut proc_macros_for_components = HashMap::new();

    for component in unit.components.iter() {
        let mut component_proc_macro_instances = Vec::new();

        for dependency in component.dependencies.iter() {
            if matches!(dependency, CompilationUnitDependency::Library(_)) {
                continue;
            }

            let Some(plugin) = unit
                .cairo_plugins
                .iter()
                .find(|plugin| &plugin.component_dependency_id == dependency.component_id())
            else {
                continue;
            };

            if plugin.builtin {
                continue;
            }

            let proc_macro = plugin.prebuilt.clone().map(Result::Ok).unwrap_or_else(|| {
                proc_macro_repository.get_or_load(plugin.package.clone(), workspace.config())
            })?;

            component_proc_macro_instances.push(proc_macro);
        }

        let proc_macro_plugin = Arc::new(ProcMacroHostPlugin::try_new(
            component_proc_macro_instances,
        )?);
        proc_macros_for_components.insert(component.id.clone(), proc_macro_plugin.clone());
    }

    Ok(proc_macros_for_components)
}
