use std::{collections::HashMap, sync::Arc};

use anyhow::Result;
use cairo_lang_semantic::{inline_macros::get_default_plugin_suite, plugin::PluginSuite};
use itertools::Itertools;
use scarb_proc_macro_server_types::scope::CompilationUnitComponent;

use crate::{
    compiler::{CairoCompilationUnit, CompilationUnitComponentId, CompilationUnitDependency},
    core::Workspace,
};

#[cfg(doc)]
use crate::core::PackageId;

use super::proc_macro::{ProcMacroHostPlugin, ProcMacroInstance};

pub struct PluginsForComponents {
    pub plugins: HashMap<CompilationUnitComponentId, PluginSuite>,
    pub proc_macros: HashMap<CompilationUnitComponentId, Arc<ProcMacroHostPlugin>>,
}

impl PluginsForComponents {
    /// Collects plugins, either built-in or procedural macros, for all components of the [`CairoCompilationUnit`].
    pub fn collect(workspace: &Workspace<'_>, unit: &CairoCompilationUnit) -> Result<Self> {
        let mut plugins = collect_builtin_plugins(workspace, unit)?;

        let proc_macros = collect_proc_macros(workspace, unit)?
            .into_iter()
            .map(|(component_id, instances)| {
                let plugin = Arc::new(ProcMacroHostPlugin::try_new(instances)?);
                Ok((component_id, plugin))
            })
            .collect::<Result<HashMap<_, _>>>()?;

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
    /// A mapping of the form: `cu_component (as a [`CompilationUnitComponent`]) -> plugin`.
    /// Contains IDs of all components of all compilation units from the workspace,
    /// each mapped to a [`ProcMacroHostPlugin`] which contains
    /// **all proc macro dependencies of the package** collected from **all compilation units it appears in**.
    pub macros_for_components: HashMap<CompilationUnitComponent, Arc<ProcMacroHostPlugin>>,
}

impl WorkspaceProcMacros {
    /// Collects and groups procedural macros for all packages in the workspace.
    pub fn collect(
        workspace: &Workspace<'_>,
        compilation_units: &[&CairoCompilationUnit],
    ) -> Result<Self> {
        let mut macros_for_components = HashMap::<_, Vec<_>>::new();

        for &unit in compilation_units {
            for (component_id, mut macro_instances) in collect_proc_macros(workspace, unit)? {
                let component: CompilationUnitComponent = unit
                    .components
                    .iter()
                    .find(|component| component.id == component_id)
                    .expect("component should always exist")
                    .into();

                macros_for_components
                    .entry(component)
                    .or_default()
                    .append(&mut macro_instances);
            }
        }

        let macros_for_components = macros_for_components
            .into_iter()
            .map(|(component, macro_instances)| {
                let deduplicated_instances = macro_instances
                    .into_iter()
                    .unique_by(|instance| instance.package_id())
                    .collect();

                let plugin = Arc::new(ProcMacroHostPlugin::try_new(deduplicated_instances)?);

                Ok((component, plugin))
            })
            .collect::<Result<HashMap<_, _>>>()?;

        Ok(Self {
            macros_for_components,
        })
    }

    /// Returns a [`ProcMacroHostPlugin`] assigned to the [`CompilationUnitComponent`].
    pub fn get(&self, component: &CompilationUnitComponent) -> Option<Arc<ProcMacroHostPlugin>> {
        self.macros_for_components.get(component).cloned()
    }
}

/// Builds [`PluginSuite`]s for each component of the [`CairoCompilationUnit`],
/// according to the dependencies on builtin macros.
fn collect_builtin_plugins(
    workspace: &Workspace<'_>,
    unit: &CairoCompilationUnit,
) -> Result<HashMap<CompilationUnitComponentId, PluginSuite>> {
    let mut plugin_suites = HashMap::new();

    for component in unit.components.iter() {
        let mut component_suite = get_default_plugin_suite();

        for dependency in component.dependencies.iter() {
            if !matches!(dependency, CompilationUnitDependency::Plugin(_)) {
                continue;
            }

            let plugin = unit
                .cairo_plugins
                .iter()
                .find(|plugin| &plugin.component_dependency_id == dependency.component_id())
                .expect("`cairo_plugins` should contain the dependency");

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

/// Collects [`ProcMacroInstances`]s for each component of the [`CairoCompilationUnit`],
/// according to the dependencies on procedural macros.
fn collect_proc_macros(
    workspace: &Workspace<'_>,
    unit: &CairoCompilationUnit,
) -> Result<HashMap<CompilationUnitComponentId, Vec<Arc<ProcMacroInstance>>>> {
    let proc_macro_repository = workspace.config().proc_macro_repository();
    let mut proc_macros_for_components = HashMap::new();

    for component in unit.components.iter() {
        let mut component_proc_macro_instances = Vec::new();

        for dependency in component.dependencies.iter() {
            if !matches!(dependency, CompilationUnitDependency::Plugin(_)) {
                continue;
            }

            let plugin = unit
                .cairo_plugins
                .iter()
                .find(|plugin| &plugin.component_dependency_id == dependency.component_id())
                .expect("`cairo_plugins` should contain the dependency");

            if plugin.builtin {
                continue;
            }

            let proc_macro = plugin.prebuilt.clone().map(Result::Ok).unwrap_or_else(|| {
                proc_macro_repository.get_or_load(plugin.package.clone(), workspace.config())
            })?;

            component_proc_macro_instances.push(proc_macro);
        }

        proc_macros_for_components.insert(component.id.clone(), component_proc_macro_instances);
    }

    Ok(proc_macros_for_components)
}
