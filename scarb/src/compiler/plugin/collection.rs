use std::{collections::HashMap, sync::Arc};

use anyhow::{Context, Result};
use cairo_lang_semantic::{inline_macros::get_default_plugin_suite, plugin::PluginSuite};

use crate::{
    compiler::{
        CairoCompilationUnit, CompilationUnitAttributes, CompilationUnitComponentId,
        CompilationUnitDependency,
    },
    core::Workspace,
};

use super::proc_macro::ProcMacroHostPlugin;

pub struct PluginsForComponents {
    pub plugins: HashMap<CompilationUnitComponentId, PluginSuite>,
    pub proc_macros: HashMap<CompilationUnitComponentId, Arc<ProcMacroHostPlugin>>,
}

impl PluginsForComponents {
    pub fn collect(workspace: &Workspace<'_>, unit: &CairoCompilationUnit) -> Result<Self> {
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

// NOTE: Since this structure is used to handle JsonRPC requests, its keys have to be serialized to strings.
//
/// A container for Proc Macro Server to manage macros present in the analyzed workspace.
pub struct WorkspaceProcMacros {
    /// A mapping of the form: serialized CompilationUnitComponentId -> plugin.
    pub macros_for_compilation_units: HashMap<String, Arc<ProcMacroHostPlugin>>,
}

impl WorkspaceProcMacros {
    /// Collects and groups procedural macros for all the components of all the compilation unit in the workspace.
    pub fn collect(
        workspace: &Workspace<'_>,
        compilation_units: &[&CairoCompilationUnit],
    ) -> Result<Self> {
        let macros_for_compilation_units = compilation_units
            .iter()
            .map(|unit| {
                let main_component = unit.main_component();
                let main_component_id = main_component.id.to_owned();
                let plugins = collect_proc_macros(workspace, unit)?
                    .get(&main_component_id)
                    .cloned()
                    .with_context(|| format!("Could not retrieve plugins for the main CU component `{main_component_id:?}`."))?;

                Ok((main_component_id.package_id.to_serialized_string(), plugins))
            })
            .collect::<Result<HashMap<_, _>>>()?;

        Ok(Self {
            macros_for_compilation_units,
        })
    }

    /// Returns a `ProcMacroHostPlugin` assigned to the component with `compilation_unit_main_package_id`.
    pub fn get(
        &self,
        compilation_unit_main_component_id: &str,
    ) -> Option<Arc<ProcMacroHostPlugin>> {
        self.macros_for_compilation_units
            .get(compilation_unit_main_component_id)
            .cloned()
    }
}

/// Builds `PluginSuite`s for each component of the [`CairoCompilationUnit`],
/// according to the dependencies on builtin macros.
fn collect_builtin_plugins(
    workspace: &Workspace<'_>,
    unit: &CairoCompilationUnit,
) -> Result<HashMap<CompilationUnitComponentId, PluginSuite>> {
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
) -> Result<HashMap<CompilationUnitComponentId, Arc<ProcMacroHostPlugin>>> {
    let proc_macro_repository = workspace.config().proc_macro_repository();
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
