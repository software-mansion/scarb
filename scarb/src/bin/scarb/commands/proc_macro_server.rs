use crate::args::ProcMacroServerArgs;
use anyhow::Result;
use itertools::Itertools;
use scarb::compiler::plugin::collection::WorkspaceProcMacros;
use scarb::ops::CompilationUnitsOpts;
use scarb::{
    compiler::CompilationUnit,
    core::Config,
    ops::{self, FeaturesOpts, FeaturesSelector},
};

#[tracing::instrument(skip_all, level = "info")]
pub fn run(args: ProcMacroServerArgs, config: &Config) -> Result<()> {
    let ws = ops::read_workspace(config.manifest_path(), config)?;
    let resolve = ops::resolve_workspace(&ws)?;
    let compilation_units = ops::generate_compilation_units(
        &resolve,
        &FeaturesOpts {
            features: FeaturesSelector::AllFeatures,
            no_default_features: false,
        },
        &ws,
        CompilationUnitsOpts {
            ignore_cairo_version: true,
            load_prebuilt_macros: !args.no_prebuilt_proc_macros,
        },
    )?;

    // Compile procedural macros only.
    for unit in &compilation_units {
        if let CompilationUnit::ProcMacro(_) = unit {
            ops::compile_unit(unit.clone(), &ws)?;
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

    ops::start_proc_macro_server(workspace_proc_macros)
}
