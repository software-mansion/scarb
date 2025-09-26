use std::sync::{Arc, Mutex};

use anyhow::anyhow;
use camino::Utf8Path;
use itertools::Itertools;
use scarb_proc_macro_server_types::methods::discover_workspace::{
    DiscoverWorkspace, DiscoverWorkspaceParams, DiscoverWorkspaceResponse,
};

use crate::{
    compiler::{self, CompilationUnit, plugin::collection::WorkspaceProcMacros},
    core::Config,
    ops::{
        self, CompilationUnitsOpts, FeaturesOpts, FeaturesSelector,
        proc_macro_server::methods::Handler, store::ProcMacroStore,
    },
};

impl Handler for DiscoverWorkspace {
    fn handle(
        config: &Config,
        proc_macros: Arc<Mutex<ProcMacroStore>>,
        params: DiscoverWorkspaceParams,
    ) -> anyhow::Result<DiscoverWorkspaceResponse> {
        let DiscoverWorkspaceParams { workspace } = params;

        let manifest_path_utf8 =
            Utf8Path::new(workspace.manifest_path.as_os_str().to_str().unwrap());
        let ws = ops::read_workspace(manifest_path_utf8, config)?;
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

        let workspace_proc_macros = WorkspaceProcMacros::collect(&ws, &cairo_compilation_units)?;
        proc_macros
            .lock()
            .map_err(|_| anyhow!("failed to acquire lock on proc macro store"))?
            .insert(workspace.clone(), workspace_proc_macros);

        Ok(DiscoverWorkspaceResponse { workspace })
    }
}
