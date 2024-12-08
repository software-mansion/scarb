use anyhow::Result;
use scarb::{
    compiler::{
        plugin::proc_macro::ProcMacroHost, CairoCompilationUnit, CompilationUnit,
        CompilationUnitAttributes,
    },
    core::{Config, PackageId, Workspace},
    ops::{self, FeaturesOpts, FeaturesSelector},
};
use std::collections::HashSet;

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
    )?;

    let mut proc_macros = ProcMacroHost::default();
    let mut loaded_plugins = HashSet::new();

    // Try loading prebuilt plugins
    for unit in &compilation_units {
        if let CompilationUnit::Cairo(unit) = unit {
            loaded_plugins.extend(load_prebuilt_plugins(unit.clone(), &ws, &mut proc_macros)?);
        }
    }

    // Then compile remaining procedural macros.
    for unit in &compilation_units {
        if let CompilationUnit::ProcMacro(_) = unit {
            if !loaded_plugins.contains(&unit.main_package_id()) {
                ops::compile_unit(unit.clone(), &ws, false)?;
            }
        }
    }

    // Load previously compiled procedural macros.
    for unit in compilation_units {
        if let CompilationUnit::Cairo(unit) = unit {
            load_plugins(unit, &ws, &mut proc_macros, &loaded_plugins)?;
        }
    }

    ops::start_proc_macro_server(proc_macros)
}

fn load_prebuilt_plugins(
    unit: CairoCompilationUnit,
    ws: &Workspace<'_>,
    proc_macros: &mut ProcMacroHost,
) -> Result<HashSet<PackageId>> {
    let mut loaded = HashSet::new();

    for plugin_info in unit.cairo_plugins.into_iter().filter(|p| !p.builtin) {
        if proc_macros
            .register_prebuilt(plugin_info.package.clone(), ws.config())
            .is_ok()
        {
            loaded.insert(plugin_info.package.id);
        }
    }

    Ok(loaded)
}

fn load_plugins(
    unit: CairoCompilationUnit,
    ws: &Workspace<'_>,
    proc_macros: &mut ProcMacroHost,
    loaded_plugins: &HashSet<PackageId>,
) -> Result<()> {
    for plugin_info in unit.cairo_plugins.into_iter().filter(|p| !p.builtin) {
        if !loaded_plugins.contains(&plugin_info.package.id) {
            proc_macros.register(plugin_info.package, ws.config())?;
        }
    }

    Ok(())
}
