use std::sync::{Arc, Mutex};

use anyhow::Result;
use camino::Utf8Path;
use itertools::Itertools;
use scarb_proc_macro_server_types::methods::defined_macros::{
    CompilationUnitComponentMacros, DebugInfo, DefinedMacros, DefinedMacrosResponse,
};

use crate::{
    compiler::{
        self, CompilationUnit,
        plugin::{collection::WorkspaceProcMacros, proc_macro::DeclaredProcMacroInstances},
    },
    core::Config,
    internal::fsx::PathUtf8Ext,
    ops::{
        self, CompilationUnitsOpts, FeaturesOpts, FeaturesSelector,
        proc_macro_server::methods::Handler, store::ProcMacroStore,
    },
};

impl Handler for DefinedMacros {
    fn handle(
        config: &Config,
        proc_macros: Arc<Mutex<ProcMacroStore>>,
        params: Self::Params,
    ) -> Result<Self::Response> {
        let Self::Params { workspace } = params;

        let manifest_path = workspace.manifest_path.try_as_utf8()?;
        let workspace_macros = get_workspace_macros(config, manifest_path)?;
        let macros_for_cu_components = get_macros_for_components(&workspace_macros);

        proc_macros
            .lock()
            .unwrap()
            .insert(workspace, workspace_macros);

        Ok(DefinedMacrosResponse {
            macros_for_cu_components,
        })
    }
}

/// Loads compiled proc macros for workspace with `manifest_path`.
fn get_workspace_macros(config: &Config, manifest_path: &Utf8Path) -> Result<WorkspaceProcMacros> {
    let ws = ops::read_workspace(manifest_path, config)?;
    let resolve = ops::resolve_workspace_with_opts(&ws, &Default::default())?;

    let compilation_units = ops::generate_compilation_units(
        &resolve,
        &FeaturesOpts {
            features: FeaturesSelector::AllFeatures,
            no_default_features: false,
        },
        &ws,
        CompilationUnitsOpts {
            ignore_cairo_version: true,
            load_prebuilt_macros: config.load_prebuilt_proc_macros(),
        },
    )?;

    // Compile procedural macros only.
    for unit in &compilation_units {
        if let CompilationUnit::ProcMacro(unit) = unit
            && unit.prebuilt.is_none()
        {
            let result = compiler::plugin::proc_macro::compile_unit(unit.clone(), &ws);
            result?;
        }
    }

    let cairo_compilation_units = compilation_units
        .iter()
        .filter_map(|unit| match unit {
            CompilationUnit::Cairo(cairo_unit) => Some(cairo_unit),
            _ => None,
        })
        .collect_vec();

    WorkspaceProcMacros::collect(&ws, &cairo_compilation_units)
}

/// Builds representations of proc macros that will be sent to LS
/// and groups them with respect to the compilation unit components that use them.
fn get_macros_for_components(
    workspace_macros: &WorkspaceProcMacros,
) -> Vec<CompilationUnitComponentMacros> {
    workspace_macros
        .macros_for_components
        .iter()
        .flat_map(|(component, plugin)| {
            plugin
                .iter()
                .map(|plugin| {
                    let attributes = plugin.declared_attributes_without_executables();
                    let inline_macros = plugin.declared_inline_macros();
                    let derives = plugin.declared_derives_snake_case();
                    let executables = plugin.executable_attributes();
                    let source_packages = plugin
                        .instances()
                        .iter()
                        .map(|instance| instance.package_id().to_serialized_string())
                        .collect();

                    CompilationUnitComponentMacros {
                        component: component.to_owned(),
                        attributes,
                        inline_macros,
                        derives,
                        executables,
                        debug_info: DebugInfo { source_packages },
                    }
                })
                .collect_vec()
        })
        .map(|cu_components_macros| (cu_components_macros.component.clone(), cu_components_macros))
        .into_group_map()
        .iter()
        .map(|(component, cu_macros)| {
            let mut derives = Vec::new();
            let mut executables = Vec::new();
            let mut attributes = Vec::new();
            let mut inline_macros = Vec::new();
            let mut source_packages = Vec::new();

            for macros in cu_macros {
                derives.extend(macros.derives.clone());
                executables.extend(macros.executables.clone());
                attributes.extend(macros.attributes.clone());
                inline_macros.extend(macros.inline_macros.clone());
                source_packages.extend(macros.debug_info.source_packages.clone());
            }

            CompilationUnitComponentMacros {
                component: component.clone(),
                executables,
                derives,
                attributes,
                inline_macros,
                debug_info: DebugInfo { source_packages },
            }
        })
        .collect()
}
