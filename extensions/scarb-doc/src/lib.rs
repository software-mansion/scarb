use crate::db::ScarbDocDatabase;
use crate::metadata::compilation::{get_project_config, get_relevant_compilation_unit};
use anyhow::Result;
use cairo_lang_compiler::diagnostics::DiagnosticsReporter;
use cairo_lang_diagnostics::{FormattedDiagnosticEntry, Severity};
use cairo_lang_filesystem::{
    db::{Edition, FilesGroup},
    ids::{CrateId, CrateLongId},
};
use errors::DiagnosticError;
use itertools::Itertools;
use scarb_metadata::{CompilationUnitMetadata, Metadata, PackageMetadata};
use scarb_ui::Ui;
use serde::Serialize;
use smol_str::ToSmolStr;
use types::Crate;

pub mod db;
pub mod docs_generation;
pub mod errors;
pub mod metadata;
pub mod types;
pub mod versioned_json_output;

#[derive(Serialize, Clone)]
pub struct PackageInformation {
    pub crate_: Crate,
    pub metadata: AdditionalMetadata,
}

#[derive(Serialize, Clone)]
pub struct AdditionalMetadata {
    pub name: String,
    pub authors: Option<Vec<String>>,
}

pub fn generate_packages_information(
    metadata: &Metadata,
    metadata_for_packages: &[PackageMetadata],
    document_private_items: bool,
    ui: Ui,
) -> Result<Vec<PackageInformation>> {
    let mut packages_information = vec![];
    for package_metadata in metadata_for_packages {
        let authors = package_metadata.manifest_metadata.authors.clone();
        let edition = package_metadata
            .edition
            .as_ref()
            .map(|edition| edition_from_string(edition))
            .transpose()?;

        let should_ignore_visibility = match edition {
            Some(edition) => edition.ignore_visibility(),
            None => Edition::default().ignore_visibility(),
        };

        let should_document_private_items = should_ignore_visibility || document_private_items;

        let compilation_unit_metadata =
            get_relevant_compilation_unit(metadata, package_metadata.id.clone())?;
        let project_config =
            get_project_config(metadata, package_metadata, compilation_unit_metadata)?;

        let db = ScarbDocDatabase::new(Some(project_config));

        let main_component = compilation_unit_metadata
            .components
            .iter()
            .find(|component| component.package == compilation_unit_metadata.package)
            .expect("main component is guaranteed to exist in compilation unit");

        let main_crate_id = db.intern_crate(CrateLongId::Real {
            name: main_component.name.to_smolstr(),
            discriminator: main_component
                .discriminator
                .as_ref()
                .map(ToSmolStr::to_smolstr),
        });

        let package_compilation_unit = metadata
            .compilation_units
            .iter()
            .find(|unit| unit.package == package_metadata.id);

        let mut diagnostics_reporter =
            setup_diagnostics_reporter(&db, main_crate_id, package_compilation_unit, &ui)
                .skip_lowering_diagnostics();

        let crate_ = Crate::new(&db, main_crate_id, should_document_private_items)
            .map_err(|_| DiagnosticError(package_metadata.name.clone()));

        if crate_.is_err() {
            diagnostics_reporter.ensure(&db)?;
        }

        packages_information.push(PackageInformation {
            crate_: crate_?,
            metadata: AdditionalMetadata {
                name: package_metadata.name.clone(),
                authors,
            },
        });
    }
    Ok(packages_information)
}

fn setup_diagnostics_reporter<'a>(
    db: &ScarbDocDatabase,
    main_crate_id: CrateId,
    package_compilation_unit: Option<&CompilationUnitMetadata>,
    ui: &'a Ui,
) -> DiagnosticsReporter<'a> {
    let ignore_warnings_crates = db
        .crates()
        .into_iter()
        .filter(|crate_id| crate_id != &main_crate_id)
        .collect_vec();

    let diagnostics_reporter = DiagnosticsReporter::callback({
        move |entry: FormattedDiagnosticEntry| {
            let msg = entry
                .message()
                .strip_suffix('\n')
                .unwrap_or(entry.message());
            match entry.severity() {
                Severity::Error => {
                    if let Some(code) = entry.error_code() {
                        ui.error_with_code(code.as_str(), msg);
                    } else {
                        ui.error(msg)
                    }
                }
                Severity::Warning => {
                    if let Some(code) = entry.error_code() {
                        ui.warn_with_code(code.as_str(), msg)
                    } else {
                        ui.warn(msg)
                    }
                }
            };
        }
    })
    .with_ignore_warnings_crates(&ignore_warnings_crates);

    // We check whether the warnings are allowed during compilation.
    match package_compilation_unit {
        Some(package_compilation_unit) => {
            if allows_warnings(package_compilation_unit) {
                return diagnostics_reporter.allow_warnings();
            }
            diagnostics_reporter
        }
        None => diagnostics_reporter,
    }
}

fn allows_warnings(compulation_unit: &CompilationUnitMetadata) -> bool {
    compulation_unit
        .compiler_config
        .as_object()
        .and_then(|config| config.get("allow_warnings"))
        .and_then(|value| value.as_bool())
        .unwrap_or(true)
}

pub fn edition_from_string(edition_str: &str) -> Result<Edition, serde_json::Error> {
    // Format `edition` to be a valid JSON string.
    let edition = format!("\"{}\"", edition_str);
    serde_json::from_str(&edition)
}
