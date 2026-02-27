//! Various utility functions helpful for interacting with Cairo compiler.

use crate::compiler::incremental::IncrementalContext;
use crate::compiler::{CairoCompilationUnit, CompilationUnitAttributes};
use crate::core::{InliningStrategy, Workspace};
use crate::flock::Filesystem;
use anyhow::{Context, Result};
use cairo_lang_compiler::CompilerConfig;
use cairo_lang_compiler::diagnostics::DiagnosticsReporter;
use cairo_lang_diagnostics::{FormattedDiagnosticEntry, Severity};
use cairo_lang_filesystem::db::FilesGroup;
use cairo_lang_filesystem::ids::CrateId;
use itertools::Itertools;
use salsa::Database;
use serde::Serialize;
use std::io::{BufWriter, Write};

pub struct CountingWriter<W> {
    inner: W,
    pub byte_count: usize,
}

impl<W: Write> CountingWriter<W> {
    pub fn new(inner: W) -> Self {
        Self {
            inner,
            byte_count: 0,
        }
    }
}

impl<W: Write> Write for CountingWriter<W> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let n = self.inner.write(buf)?;
        self.byte_count += n;
        Ok(n)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.inner.flush()
    }
}

pub fn build_compiler_config<'c, 'db>(
    db: &'db dyn Database,
    unit: &CairoCompilationUnit,
    main_crate_ids: &[CrateId<'db>],
    ctx: &'db IncrementalContext,
    ws: &Workspace<'c>,
) -> CompilerConfig<'c>
where
    'db: 'c,
{
    let ignore_warnings_crates = db
        .crates()
        .iter()
        .filter(|crate_id| !main_crate_ids.contains(crate_id))
        .map(|c| c.long(db).clone().into_crate_input(db))
        .collect_vec();
    // If a crate is cached, we do not need to check it for error diagnostics,
    // as the cache can only be produced if the crate is error-free.
    // So if there were any diagnostics here to show, it would mean that the cache is outdated - thus
    // we should not use it in the first place.
    // We also skip showing warnings produced for dependency crates.
    let crates_to_check = db.crates().iter().filter(|crate_id| {
        let crate_input = crate_id.long(db).clone().into_crate_input(db);
        if ctx.cached_crates().contains(&crate_input) {
            // Even if cached, include the main crate in diagnostics checking when its
            // warnings are unknown (i.e., it was previously compiled only as a dependency).
            main_crate_ids.contains(crate_id)
                && ctx.cached_crate_warnings_for(&crate_input).is_none()
        } else {
            true
        }
    });
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
                    let code = entry.error_code().map(|c| c.as_str().to_string());
                    ctx.add_warning(code.clone(), msg.to_string());
                    if let Some(code) = code {
                        config.ui().warn_with_code(&code, msg)
                    } else {
                        config.ui().warn(msg)
                    }
                }
            };
        }
    })
    .with_ignore_warnings_crates(&ignore_warnings_crates)
    .with_crates(
        &crates_to_check
            .into_iter()
            .map(|c| c.long(db).clone().into_crate_input(db))
            .collect_vec(),
    );
    CompilerConfig {
        diagnostics_reporter: if unit.compiler_config.allow_warnings {
            diagnostics_reporter.allow_warnings()
        } else {
            diagnostics_reporter
        },
        replace_ids: unit.compiler_config.sierra_replace_ids,
        add_statements_functions: unit.compiler_config.add_statements_functions_debug_info,
        add_statements_code_locations: unit
            .compiler_config
            .add_statements_code_locations_debug_info,
        add_functions_debug_info: unit.compiler_config.add_functions_debug_info,
    }
}

impl From<InliningStrategy> for cairo_lang_lowering::utils::InliningStrategy {
    fn from(value: InliningStrategy) -> Self {
        match value {
            InliningStrategy::Default => cairo_lang_lowering::utils::InliningStrategy::Default,
            InliningStrategy::Avoid => cairo_lang_lowering::utils::InliningStrategy::Avoid,
            InliningStrategy::InlineSmallFunctions(weight) => {
                cairo_lang_lowering::utils::InliningStrategy::InlineSmallFunctions(weight)
            }
        }
    }
}

#[allow(unused)]
impl From<cairo_lang_lowering::utils::InliningStrategy> for InliningStrategy {
    fn from(value: cairo_lang_lowering::utils::InliningStrategy) -> Self {
        match value {
            cairo_lang_lowering::utils::InliningStrategy::Default => InliningStrategy::Default,
            cairo_lang_lowering::utils::InliningStrategy::Avoid => InliningStrategy::Avoid,
            cairo_lang_lowering::utils::InliningStrategy::InlineSmallFunctions(weight) => {
                InliningStrategy::InlineSmallFunctions(weight)
            }
        }
    }
}

pub fn collect_main_crate_ids<'db>(
    unit: &CairoCompilationUnit,
    db: &'db dyn Database,
) -> Vec<CrateId<'db>> {
    let main_component = unit.main_component();
    vec![main_component.crate_id(db)]
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

pub fn write_json_with_byte_count(
    file_name: &str,
    description: &str,
    target_dir: &Filesystem,
    ws: &Workspace<'_>,
    value: impl Serialize,
) -> Result<usize> {
    let file = target_dir.create_rw(file_name, description, ws.config())?;
    let file = BufWriter::new(&*file);
    let mut writer = CountingWriter::new(file);
    serde_json::to_writer(&mut writer, &value)
        .with_context(|| format!("failed to serialize {file_name}"))?;
    Ok(writer.byte_count)
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
