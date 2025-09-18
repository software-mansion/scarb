use std::sync::{Arc, Mutex};

use anyhow::anyhow;
use camino::Utf8Path;
use itertools::Itertools;
use scarb_proc_macro_server_types::methods::discover_workspace::{
    DiscoverWorkspaceParams, DiscoverWorkspaceResponse,
};
use tracing::trace;

use crate::{
    compiler::{
        self, CompilationUnit, CompilationUnitAttributes, plugin::collection::WorkspaceProcMacros,
    },
    core::Config,
    ops::{self, CompilationUnitsOpts, FeaturesOpts, FeaturesSelector, store::ProcMacroStore},
};

pub fn discover_workspace(
    config: &Config,
    macro_store: Arc<Mutex<ProcMacroStore>>,
    params: DiscoverWorkspaceParams,
) -> anyhow::Result<DiscoverWorkspaceResponse> {
    let DiscoverWorkspaceParams { workspace } = params;
    trace!(
        "[PMS] Running discoverWorkspace at {}",
        workspace.manifest_path.to_string_lossy().to_string()
    );

    let manifest_path_utf8 = Utf8Path::new(workspace.manifest_path.as_os_str().to_str().unwrap());
    let ws = ops::read_workspace(manifest_path_utf8, config)?;
    let resolve = ops::resolve_workspace_with_opts(&ws, &Default::default())?;

    trace!("[PMS] Workspace resolved");

    trace!("[PMS] Generating compilation units...");
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

    trace!("[PMS] Compilation units generated");

    // Compile procedural macros only.
    for unit in &compilation_units {
        if let CompilationUnit::ProcMacro(unit) = unit
            && unit.prebuilt.is_none()
        {
            trace!("[PMS] compiling unit {}", unit.main_package_id().name);
            let result = compiler::plugin::proc_macro::compile_unit(unit.clone(), &ws);
            trace!("[PMS] compilation result: {result:?}");
            result?;
        }
    }

    trace!("[PMS] Macros compiled");

    let cairo_compilation_units = compilation_units
        .iter()
        .filter_map(|unit| match unit {
            CompilationUnit::Cairo(cairo_unit) => Some(cairo_unit),
            _ => None,
        })
        .collect_vec();

    let workspace_proc_macros = WorkspaceProcMacros::collect(&ws, &cairo_compilation_units)?;
    trace!(
        "[PMS] {} macros loaded for workspace {}",
        workspace_proc_macros
            .macros_for_components
            .iter()
            .map(|macros_for_cu| macros_for_cu.1.len())
            .sum::<usize>(),
        workspace.manifest_path.to_string_lossy().to_string()
    );
    macro_store
        .lock()
        .map_err(|_| anyhow!("failed to acquire lock on proc macro store"))?
        .insert(workspace.clone(), workspace_proc_macros);

    Ok(DiscoverWorkspaceResponse { workspace })
}
