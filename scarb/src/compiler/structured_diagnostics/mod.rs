mod core;
mod scarb_ui;

use crate::core::Workspace;
use cairo_lang_compiler::diagnostics::DiagnosticsError;
use cairo_lang_filesystem::db::FilesGroup;
use cairo_lang_filesystem::ids::CrateId;
use itertools::Itertools;
use salsa::Database;

use self::core::StructuredDiagnosticsReporter;
use self::scarb_ui::ScarbUiStructuredDiagnosticsSink;

pub fn ensure_structured_json_diagnostics<'db>(
    db: &'db dyn Database,
    main_crate_ids: &[CrateId<'db>],
    ws: &Workspace<'_>,
) -> std::result::Result<(), DiagnosticsError> {
    let ignore_warnings_crates = db
        .crates()
        .iter()
        .filter(|crate_id| !main_crate_ids.contains(crate_id))
        .map(|c| c.long(db).clone().into_crate_input(db))
        .collect_vec();
    let crates_to_check = db
        .crates()
        .iter()
        .map(|c| c.long(db).clone().into_crate_input(db))
        .collect_vec();
    let mut sink = ScarbUiStructuredDiagnosticsSink::new(ws.config().ui().clone());
    let mut reporter = StructuredDiagnosticsReporter::new(ignore_warnings_crates, crates_to_check);
    if reporter.check(db, &mut sink) {
        Err(DiagnosticsError)
    } else {
        Ok(())
    }
}
