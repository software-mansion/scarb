use cairo_lang_defs::db::DefsGroup;
use cairo_lang_defs::ids::ModuleId;
use cairo_lang_diagnostics::{
    DiagnosticEntry, Diagnostics, PluginFileDiagnosticNotes, Severity, UserLocationWithPluginNotes,
};
use cairo_lang_filesystem::db::FilesGroup;
use cairo_lang_filesystem::ids::{CrateInput, SpanInFile};
use cairo_lang_lowering::db::LoweringGroup;
use cairo_lang_parser::db::ParserGroup;
use cairo_lang_semantic::db::SemanticGroup;
use cairo_lang_utils::Intern;
use cairo_lang_utils::unordered_hash_set::UnorderedHashSet;
use itertools::Itertools;
use salsa::Database;
use serde::Serialize;

#[derive(Serialize)]
pub struct StructuredDiagnosticMessage {
    r#type: &'static str,
    severity: StructuredDiagnosticSeverity,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    code: Option<String>,
    file: String,
    span: StructuredDiagnosticSpan,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    related: Vec<StructuredDiagnosticRelated>,
}

#[derive(Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum StructuredDiagnosticSeverity {
    Error,
    Warning,
}

struct StructuredDiagnosticLocation {
    file: String,
    span: StructuredDiagnosticSpan,
}

#[derive(Serialize)]
struct StructuredDiagnosticSpan {
    start: usize,
    end: usize,
}

#[derive(Serialize)]
struct StructuredDiagnosticRelated {
    message: String,
    file: String,
    span: StructuredDiagnosticSpan,
}

pub trait StructuredDiagnosticsSink {
    fn emit(&mut self, message: StructuredDiagnosticMessage);
}

pub struct StructuredDiagnosticsReporter {
    ignore_warnings_crate_ids: Vec<CrateInput>,
    crates: Vec<CrateInput>,
}

impl StructuredDiagnosticsReporter {
    pub fn new(ignore_warnings_crate_ids: Vec<CrateInput>, crates: Vec<CrateInput>) -> Self {
        Self {
            ignore_warnings_crate_ids,
            crates,
        }
    }

    pub fn check(&mut self, db: &dyn Database, sink: &mut impl StructuredDiagnosticsSink) -> bool {
        let mut found_diagnostics = false;

        for crate_input in self.crates.clone() {
            let crate_id = crate_input.clone().into_crate_long_id(db).intern(db);
            let Ok(module_file) = db.module_main_file(ModuleId::CrateRoot(crate_id)) else {
                found_diagnostics = true;
                sink.emit(StructuredDiagnosticMessage::error(
                    "Failed to get main module file".to_string(),
                    "<unknown>".to_string(),
                ));
                continue;
            };

            if db.file_content(module_file).is_none() {
                let file = module_file.full_path(db);
                sink.emit(StructuredDiagnosticMessage::error(
                    format!("{file} not found"),
                    file,
                ));
                found_diagnostics = true;
            }

            let skip_warnings = self.ignore_warnings_crate_ids.contains(&crate_input);
            let modules = db.crate_modules(crate_id);
            let mut processed_file_ids = UnorderedHashSet::<_>::default();
            for module_id in modules.iter() {
                let default = Default::default();
                let diagnostic_notes = module_id
                    .module_data(db)
                    .map(|data| data.diagnostics_notes(db))
                    .unwrap_or(&default);

                if let Ok(module_files) = db.module_files(*module_id) {
                    for file_id in module_files.iter().copied() {
                        if processed_file_ids.insert(file_id) {
                            found_diagnostics |= self.check_diag_group(
                                db.as_dyn_database(),
                                db.file_syntax_diagnostics(file_id).clone(),
                                skip_warnings,
                                diagnostic_notes,
                                sink,
                            );
                        }
                    }
                }

                if let Ok(group) = db.module_semantic_diagnostics(*module_id) {
                    found_diagnostics |= self.check_diag_group(
                        db.as_dyn_database(),
                        group,
                        skip_warnings,
                        diagnostic_notes,
                        sink,
                    );
                }

                if let Ok(group) = db.module_lowering_diagnostics(*module_id) {
                    found_diagnostics |= self.check_diag_group(
                        db.as_dyn_database(),
                        group,
                        skip_warnings,
                        diagnostic_notes,
                        sink,
                    );
                }
            }
        }

        found_diagnostics
    }

    fn check_diag_group<'db, TEntry: DiagnosticEntry<'db> + salsa::Update>(
        &mut self,
        db: &'db dyn Database,
        group: Diagnostics<'db, TEntry>,
        skip_warnings: bool,
        file_notes: &PluginFileDiagnosticNotes<'db>,
        sink: &mut impl StructuredDiagnosticsSink,
    ) -> bool {
        let mut found = false;
        for entry in group.get_diagnostics_without_duplicates(db) {
            if skip_warnings && entry.severity() == Severity::Warning {
                continue;
            }

            if let Some(message) = build_structured_diagnostic_message(db, &entry, file_notes) {
                sink.emit(message);
                found |= group.check_error_free().is_err();
            }
        }
        found
    }
}

impl StructuredDiagnosticMessage {
    fn error(message: String, file: String) -> Self {
        Self {
            r#type: "diagnostic",
            severity: StructuredDiagnosticSeverity::Error,
            message,
            code: None,
            file,
            span: StructuredDiagnosticSpan { start: 0, end: 0 },
            related: vec![],
        }
    }

    pub fn severity(&self) -> StructuredDiagnosticSeverity {
        self.severity
    }
}

impl StructuredDiagnosticLocation {
    fn from_user_location(db: &dyn Database, location: SpanInFile<'_>) -> Self {
        Self {
            file: location.file_id.full_path(db),
            span: StructuredDiagnosticSpan {
                start: location.span.start.as_u32() as usize,
                end: location.span.end.as_u32() as usize,
            },
        }
    }

    fn into_related(self, message: String) -> StructuredDiagnosticRelated {
        StructuredDiagnosticRelated {
            message,
            file: self.file,
            span: self.span,
        }
    }
}

fn build_structured_diagnostic_message<'db, TEntry: DiagnosticEntry<'db>>(
    db: &'db dyn Database,
    entry: &TEntry,
    file_notes: &PluginFileDiagnosticNotes<'db>,
) -> Option<StructuredDiagnosticMessage> {
    let diag_location = entry.location(db);
    let (user_location, parent_file_notes) =
        diag_location.user_location_with_plugin_notes(db, file_notes);
    let primary = StructuredDiagnosticLocation::from_user_location(db, user_location);

    let mut related = entry
        .notes(db)
        .iter()
        .chain(parent_file_notes.iter())
        .filter_map(|note| {
            note.location.map(|location| {
                StructuredDiagnosticLocation::from_user_location(db, location.user_location(db))
                    .into_related(note.text.clone())
            })
        })
        .collect_vec();

    if diag_location != user_location {
        related.push(
            StructuredDiagnosticLocation::from_user_location(db, diag_location)
                .into_related("diagnostic originates in generated code".to_string()),
        );
    }

    Some(StructuredDiagnosticMessage {
        r#type: "diagnostic",
        severity: match entry.severity() {
            Severity::Error => StructuredDiagnosticSeverity::Error,
            Severity::Warning => StructuredDiagnosticSeverity::Warning,
        },
        message: entry.format(db),
        code: entry.error_code().map(|code| code.to_string()),
        file: primary.file,
        span: primary.span,
        related,
    })
}
