use anyhow::Result;
use scarb::{
    compiler::{plugin::proc_macro::ProcMacroHost, CairoCompilationUnit, CompilationUnit},
    core::{Config, Workspace},
    ops::{self, FeaturesOpts, FeaturesSelector},
};

#[tracing::instrument(skip_all, level = "info")]
pub fn run(config: &mut Config) -> Result<()> {
    let ws = ops::read_workspace(config.manifest_path(), config)?;
    let resolve = ops::resolve_workspace(&ws)?;
    let compilation_units = ops::generate_compilation_units(
        &resolve,
        &FeaturesOpts {
            features: FeaturesSelector::AllFeatures,
            no_default_features: false,
        },
        true,
        &ws,
        true,
    )?;

    // Compile procedural macros only.
    for unit in &compilation_units {
        if let CompilationUnit::ProcMacro(_) = unit {
            ops::compile_unit(unit.clone(), &ws)?;
        }
    }

    let mut proc_macros = ProcMacroHost::default();

    // Load previously compiled procedural macros.
    for unit in compilation_units {
        if let CompilationUnit::Cairo(unit) = unit {
            load_plugins(unit, &ws, &mut proc_macros)?;
        }
    }

    ops::start_proc_macro_server(proc_macros)
}

fn load_plugins(
    unit: CairoCompilationUnit,
    ws: &Workspace<'_>,
    proc_macros: &mut ProcMacroHost,
) -> Result<()> {
    for plugin_info in unit.cairo_plugins.into_iter().filter(|p| !p.builtin) {
        if let Some(prebuilt) = plugin_info.prebuilt {
            proc_macros.register_instance(prebuilt);
        } else {
            proc_macros.register_new(plugin_info.package, ws.config())?;
        }
    }

    Ok(())
}
