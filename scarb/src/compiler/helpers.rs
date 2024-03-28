//! Various utility functions helpful for interacting with Cairo compiler.

use anyhow::{Context, Result};
use cairo_lang_compiler::db::RootDatabase;
use cairo_lang_compiler::diagnostics::DiagnosticsReporter;
use cairo_lang_compiler::CompilerConfig;
use cairo_lang_diagnostics::{FormattedDiagnosticEntry, Severity};
use cairo_lang_filesystem::db::FilesGroup;
use cairo_lang_filesystem::ids::{CrateId, CrateLongId};
use serde::Serialize;
use std::io::{BufWriter, Write};

use crate::compiler::{CairoCompilationUnit, CompilationUnitAttributes};
use crate::core::Workspace;
use crate::flock::Filesystem;

pub fn build_compiler_config<'c>(
    unit: &CairoCompilationUnit,
    ws: &Workspace<'c>,
) -> CompilerConfig<'c> {
    let diagnostics_reporter = DiagnosticsReporter::callback({
        let config = ws.config();

        |entry: FormattedDiagnosticEntry| {
            let msg = entry
                .message()
                .strip_suffix('\n')
                .unwrap_or(entry.message());
            match entry.severity() {
                Severity::Error => config.ui().error(msg),
                Severity::Warning => config.ui().warn(msg),
            };
        }
    });
    CompilerConfig {
        diagnostics_reporter: if unit.compiler_config.allow_warnings {
            diagnostics_reporter.allow_warnings()
        } else {
            diagnostics_reporter
        },
        replace_ids: unit.compiler_config.sierra_replace_ids,
        add_statements_functions: unit.compiler_config.add_statements_functions_debug_info,
        ..CompilerConfig::default()
    }
}

pub fn collect_main_crate_ids(unit: &CairoCompilationUnit, db: &RootDatabase) -> Vec<CrateId> {
    vec![db.intern_crate(CrateLongId::Real(
        unit.main_component().cairo_package_name(),
    ))]
}

pub fn collect_all_crate_ids(unit: &CairoCompilationUnit, db: &RootDatabase) -> Vec<CrateId> {
    unit.components
        .iter()
        .map(|component| db.intern_crate(CrateLongId::Real(component.cairo_package_name())))
        .collect()
}

pub fn write_json(
    file_name: &str,
    description: &str,
    target_dir: &Filesystem,
    ws: &Workspace<'_>,
    value: impl Serialize,
) -> Result<()> {
    let file = target_dir.open_rw(file_name, description, ws.config())?;
    let file = BufWriter::new(&*file);
    serde_json::to_writer(file, &value)
        .with_context(|| format!("failed to serialize {file_name}"))?;
    Ok(())
}

pub fn write_string(
    file_name: &str,
    description: &str,
    target_dir: &Filesystem,
    ws: &Workspace<'_>,
    value: impl ToString,
) -> Result<()> {
    let mut file = target_dir.open_rw(file_name, description, ws.config())?;
    file.write_all(value.to_string().as_bytes())?;
    Ok(())
}
