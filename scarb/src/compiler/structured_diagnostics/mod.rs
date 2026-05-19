mod core;
mod scarb_ui;

use crate::compiler::helpers::{all_crate_inputs, non_main_crate_inputs};
use crate::core::Workspace;
use cairo_lang_compiler::diagnostics::DiagnosticsError;
use cairo_lang_compiler::diagnostics::DiagnosticsReporter;
use cairo_lang_compiler::ensure_diagnostics;
use cairo_lang_filesystem::ids::CrateId;
use cairo_lang_utils::CloneableDatabase;

use self::core::StructuredDiagnosticsReporter;
use self::scarb_ui::ScarbUiStructuredDiagnosticsSink;

pub fn ensure_structured_json_diagnostics<'db>(
    db: &'db dyn CloneableDatabase,
    main_crate_ids: &[CrateId<'db>],
    ws: &Workspace<'_>,
) -> Result<(), DiagnosticsError> {
    let ignore_warnings_crates = non_main_crate_inputs(db, main_crate_ids);
    let crates_to_check = all_crate_inputs(db);
    let mut sink = ScarbUiStructuredDiagnosticsSink::new(ws.config().ui().clone());
    let reporter =
        StructuredDiagnosticsReporter::new(ignore_warnings_crates, crates_to_check.clone());
    let mut warmup_reporter = DiagnosticsReporter::ignoring().with_crates(&crates_to_check);
    let _ = ensure_diagnostics(db, &mut warmup_reporter);

    if reporter.check(db, &mut sink) {
        Err(DiagnosticsError)
    } else {
        Ok(())
    }
}
