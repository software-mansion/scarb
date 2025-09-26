use crate::compiler::plugin::proc_macro::InstanceLoader;
use std::{collections::HashMap, sync::Arc};

use anyhow::{Result, ensure};
use cairo_lang_plugins::plugins::ConfigPlugin;
use cairo_lang_semantic::{inline_macros::get_default_plugin_suite, plugin::PluginSuite};
use itertools::Itertools;
use scarb_proc_macro_server_types::scope::CompilationUnitComponent;
use smol_str::SmolStr;
use std::vec::IntoIter;

use super::proc_macro::{DeclaredProcMacroInstances, ProcMacroHostPlugin, ProcMacroInstance};
use crate::compiler::plugin::CairoPlugin;
use crate::core::PackageId;
use crate::{
    compiler::{CairoCompilationUnit, CompilationUnitComponentId, CompilationUnitDependency},
    core::Workspace,
};

pub struct PluginsForComponents {
    pub plugins: HashMap<CompilationUnitComponentId, PluginSuite>,
    pub proc_macros: HashMap<CompilationUnitComponentId, ComponentProcMacroHost>,
}

impl PluginsForComponents {
    /// Collects plugins, either built-in or procedural macros, for all components of the [`CairoCompilationUnit`].
    pub fn collect(workspace: &Workspace<'_>, unit: &CairoCompilationUnit) -> Result<Self> {
        let mut plugins = collect_builtin_plugins(workspace, unit)?;

        let proc_macros = collect_proc_macros(workspace, unit)?
            .into_iter()
            .map(|(component_id, instances)| {
                Ok((
                    component_id,
                    ComponentProcMacroHost::try_from_instances(instances)?,
                ))
            })
            .collect::<Result<HashMap<_, _>>>()?;

        for (component_id, suite) in plugins.iter_mut() {
            if let Some(proc_macro) = proc_macros.get(component_id) {
                suite.add_proc_macro(proc_macro.build_plugin_suite());
            }
        }

        let plugins = plugins
            .into_iter()
            .map(|(id, suite)| (id, suite.assemble()))
            .collect();

        Ok(Self {
            plugins,
            proc_macros,
        })
    }
}

// NOTE: Since this structure is used to handle JsonRPC requests, its keys have to be serialized to strings.
//
/// A container for Proc Macro Server to manage macros present in the analyzed workspace.
#[derive(Default)]
pub struct WorkspaceProcMacros {
    /// A mapping of the form: `cu_component (as a [`CompilationUnitComponent`]) -> plugin`.
    /// Contains IDs of all components of all compilation units from the workspace,
    /// each mapped to a [`ProcMacroHostPlugin`] which contains
    /// **all proc macro dependencies of the package** collected from **all compilation units it appears in**.
    pub macros_for_components: HashMap<CompilationUnitComponent, Arc<Vec<ProcMacroHostPlugin>>>,
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
                let deduplicated_instances: Vec<Arc<ProcMacroInstance>> = macro_instances
                    .into_iter()
                    .unique_by(|instance| instance.package_id())
                    .collect();
                let proc_macros =
                    ComponentProcMacroHost::try_from_instances(deduplicated_instances)?;
                let proc_macros: Vec<ProcMacroHostPlugin> = proc_macros.into();
                Ok((component, Arc::new(proc_macros)))
            })
            .collect::<Result<HashMap<_, _>>>()?;

        Ok(Self {
            macros_for_components,
        })
    }

    /// Returns a vector of [`ProcMacroHostPlugin`]s assigned to the [`CompilationUnitComponent`].
    ///
    /// Proc macro instances should be grouped into separate plugins by macro api version used.
    pub fn get(
        &self,
        component: &CompilationUnitComponent,
    ) -> Option<Arc<Vec<ProcMacroHostPlugin>>> {
        self.macros_for_components.get(component).cloned()
    }
}

/// Builds [`PluginSuite`]s for each component of the [`CairoCompilationUnit`],
/// according to the dependencies on builtin macros.
fn collect_builtin_plugins(
    workspace: &Workspace<'_>,
    unit: &CairoCompilationUnit,
) -> Result<HashMap<CompilationUnitComponentId, PluginSuiteAssembler>> {
    let mut plugin_suites = HashMap::new();

    for component in unit.components.iter() {
        let mut component_suite = PluginSuiteAssembler::default();

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
            component_suite.add_builtin(plugin)?;
        }

        plugin_suites.insert(component.id.clone(), component_suite);
    }

    Ok(plugin_suites)
}

#[derive(Default)]
struct PluginSuiteAssembler {
    builtin: PluginSuite,
    proc_macro: PluginSuite,
}

impl PluginSuiteAssembler {
    pub fn add_builtin(&mut self, plugin: &dyn CairoPlugin) -> Result<()> {
        let instance = plugin.instantiate()?;
        let suite = instance.plugin_suite();
        self.builtin.add(suite);
        Ok(())
    }
    pub fn add_proc_macro(&mut self, suite: PluginSuite) {
        self.proc_macro.add(suite);
    }
    pub fn assemble(self) -> PluginSuite {
        let mut suite = PluginSuite::default();
        suite
            // Config plugin must be first, as it removes items from the AST,
            // and other plugins may add items prior to the removal of the original.
            .add_plugin::<ConfigPlugin>()
            .add(self.proc_macro)
            // The default plugin suite also contains `ConfigPlugin`, but running it twice
            // has negligible performance cost.
            .add(get_default_plugin_suite())
            .add(self.builtin);
        suite
    }
}

/// Collects [`ProcMacroInstances`]s for each component of the [`CairoCompilationUnit`],
/// according to the dependencies on procedural macros.
fn collect_proc_macros(
    workspace: &Workspace<'_>,
    unit: &CairoCompilationUnit,
) -> Result<HashMap<CompilationUnitComponentId, Vec<Arc<ProcMacroInstance>>>> {
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

            component_proc_macro_instances.push(plugin.instantiate(workspace.config())?);
        }

        proc_macros_for_components.insert(component.id.clone(), component_proc_macro_instances);
    }

    Ok(proc_macros_for_components)
}

pub struct ComponentProcMacroHost(Vec<ProcMacroHostPlugin>);

impl ComponentProcMacroHost {
    pub fn try_new(hosts: Vec<ProcMacroHostPlugin>) -> Result<Self> {
        struct MacroId {
            package_id: PackageId,
            expansion_name: SmolStr,
        }

        // Validate expansions across hosts.
        let mut expansions = hosts
            .iter()
            .flat_map(|host| host.instances())
            .flat_map(|m| {
                m.get_expansions()
                    .iter()
                    .map(|e| MacroId {
                        package_id: m.package_id(),
                        expansion_name: e.cairo_name.clone(),
                    })
                    .collect_vec()
            })
            .collect::<Vec<_>>();
        expansions.sort_unstable_by_key(|e| (e.expansion_name.clone(), e.package_id));
        ensure!(
            expansions
                .windows(2)
                .all(|w| w[0].expansion_name != w[1].expansion_name),
            "duplicate expansions defined for procedural macros: {duplicates}",
            duplicates = expansions
                .windows(2)
                .filter(|w| w[0].expansion_name == w[1].expansion_name)
                .map(|w| format!(
                    "{} ({} and {})",
                    w[0].expansion_name.as_str(),
                    w[0].package_id,
                    w[1].package_id
                ))
                .collect::<Vec<_>>()
                .join(", ")
        );
        Ok(Self(hosts))
    }

    pub fn try_from_instances(instances: Vec<Arc<ProcMacroInstance>>) -> Result<Self> {
        let instances = instances
            .into_iter()
            .sorted_by_key(|instance| instance.api_version())
            .chunk_by(|instance| instance.api_version());
        let plugins = instances
            .into_iter()
            .map(|(api_version, instances)| {
                let instances: Vec<Arc<ProcMacroInstance>> = instances.collect_vec();
                ProcMacroHostPlugin::try_new(instances, api_version)
            })
            .collect::<Result<Vec<ProcMacroHostPlugin>>>()?;
        Self::try_new(plugins)
    }

    pub fn build_plugin_suite(&self) -> PluginSuite {
        let mut suite = PluginSuite::default();
        for host in self.0.iter() {
            suite.add(host.build_plugin_suite());
        }
        suite
    }
}

impl IntoIterator for ComponentProcMacroHost {
    type Item = ProcMacroHostPlugin;
    type IntoIter = IntoIter<Self::Item>;

    fn into_iter(self) -> IntoIter<ProcMacroHostPlugin> {
        self.0.into_iter()
    }
}

impl From<ComponentProcMacroHost> for Vec<ProcMacroHostPlugin> {
    fn from(host: ComponentProcMacroHost) -> Self {
        host.0
    }
}
