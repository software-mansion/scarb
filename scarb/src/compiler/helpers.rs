//! Various utility functions helpful for interacting with Cairo compiler.

use crate::compiler::{CairoCompilationUnit, CompilationUnitAttributes};
use crate::core::{InliningStrategy, Workspace};
use crate::flock::Filesystem;
use anyhow::{Context, Result};
use cairo_lang_compiler::db::RootDatabase;
use cairo_lang_compiler::diagnostics::DiagnosticsReporter;
use cairo_lang_compiler::CompilerConfig;
use cairo_lang_diagnostics::{FormattedDiagnosticEntry, Severity};
use cairo_lang_filesystem::db::FilesGroup;
use cairo_lang_filesystem::ids::{CrateId, CrateLongId};
use itertools::Itertools;
use serde::Serialize;
use std::io::{BufWriter, Write};

pub fn build_compiler_config<'c>(
    db: &RootDatabase,
    unit: &CairoCompilationUnit,
    main_crate_ids: &[CrateId],
    ws: &Workspace<'c>,
) -> CompilerConfig<'c> {
    let ignore_warnings_crates = db
        .crates()
        .into_iter()
        .filter(|crate_id| !main_crate_ids.contains(crate_id))
        .collect_vec();
    let diagnostics_reporter = DiagnosticsReporter::callback({
        let config = ws.config();

        |entry: FormattedDiagnosticEntry| {
            let msg = entry
                .message()
                .strip_suffix('\n')
                .unwrap_or(entry.message());
            match entry.severity() {
                Severity::Error => {
                    if let Some(code) = entry.error_code() {
                        config.ui().error_with_code(code.as_str(), msg)
                    } else {
                        config.ui().error(msg)
                    }
                }
                Severity::Warning => {
                    if let Some(code) = entry.error_code() {
                        config.ui().warn_with_code(code.as_str(), msg)
                    } else {
                        config.ui().warn(msg)
                    }
                }
            };
        }
    })
    .with_ignore_warnings_crates(&ignore_warnings_crates);
    CompilerConfig {
        diagnostics_reporter: if unit.compiler_config.allow_warnings {
            diagnostics_reporter.allow_warnings()
        } else {
            diagnostics_reporter
        },
        replace_ids: unit.compiler_config.sierra_replace_ids,
        inlining_strategy: unit.compiler_config.inlining_strategy.clone().into(),
        add_statements_functions: unit
            .compiler_config
            .unstable_add_statements_functions_debug_info,
        add_statements_code_locations: unit
            .compiler_config
            .unstable_add_statements_code_locations_debug_info,
        ..CompilerConfig::default()
    }
}

impl From<InliningStrategy> for cairo_lang_lowering::utils::InliningStrategy {
    fn from(value: InliningStrategy) -> Self {
        match value {
            InliningStrategy::Default => cairo_lang_lowering::utils::InliningStrategy::Default,
            InliningStrategy::Avoid => cairo_lang_lowering::utils::InliningStrategy::Avoid,
        }
    }
}

#[allow(unused)]
impl From<cairo_lang_lowering::utils::InliningStrategy> for InliningStrategy {
    fn from(value: cairo_lang_lowering::utils::InliningStrategy) -> Self {
        match value {
            cairo_lang_lowering::utils::InliningStrategy::Default => InliningStrategy::Default,
            cairo_lang_lowering::utils::InliningStrategy::Avoid => InliningStrategy::Avoid,
        }
    }
}

pub fn collect_main_crate_ids(unit: &CairoCompilationUnit, db: &RootDatabase) -> Vec<CrateId> {
    let main_component = unit.main_component();
    let name = main_component.cairo_package_name();
    vec![db.intern_crate(CrateLongId::Real {
        discriminator: main_component.id.to_discriminator(),
        name,
    })]
}

pub fn write_json(
    file_name: &str,
    description: &str,
    target_dir: &Filesystem,
    ws: &Workspace<'_>,
    value: impl Serialize,
) -> Result<()> {
    let file = target_dir.create_rw(file_name, description, ws.config())?;
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
    let mut file = target_dir.create_rw(file_name, description, ws.config())?;
    file.write_all(value.to_string().as_bytes())?;
    Ok(())
}
