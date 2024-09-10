use anyhow::Result;
use cairo_lang_compiler::diagnostics::DiagnosticsReporter;
use cairo_lang_compiler::project::ProjectConfig;
use cairo_lang_diagnostics::{FormattedDiagnosticEntry, Maybe, Severity};
use cairo_lang_filesystem::db::{Edition, FilesGroup};
use cairo_lang_filesystem::ids::CrateLongId;
use cairo_lang_utils::Upcast;
use errors::DiagnosticError;
use scarb_metadata::{Metadata, PackageMetadata};
use serde::Serialize;

use crate::db::ScarbDocDatabase;
use crate::metadata::compilation::get_project_config;
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

        let project_config = get_project_config(metadata, package_metadata)?;

        let crate_ = generate_language_elements_tree_for_package(
            package_metadata.name.clone(),
            project_config,
            should_document_private_items,
        )
        .map_err(|_| DiagnosticError(package_metadata.name.clone()))?;

        packages_information.push(PackageInformation {
            crate_,
            metadata: AdditionalMetadata {
                name: package_metadata.name.clone(),
                authors,
            },
        });
    }
    Ok(packages_information)
}

fn generate_language_elements_tree_for_package(
    package_name: String,
    project_config: ProjectConfig,
    document_private_items: bool,
) -> Maybe<Crate> {
    let db = ScarbDocDatabase::new(Some(project_config))?;

    let main_crate_id = db.db.intern_crate(CrateLongId::Real(package_name.into()));

    let mut diagnostics_reporter = setup_diagnostics_reporter();

    diagnostics_reporter.ensure(&db.db);

    Crate::new(&db, main_crate_id, document_private_items)
}

fn setup_diagnostics_reporter() -> DiagnosticsReporter {
    DiagnosticsReporter::callback({
        |entry: FormattedDiagnosticEntry| {
            let msg = entry
                .message()
                .strip_suffix('\n')
                .unwrap_or(entry.message());
            match entry.severity() {
                Severity::Error => {
                    if let Some(code) = entry.error_code() {
                        // config.ui().error_with_code(code.as_str(), msg)
                        eprintln!("{} {}", code.as_str(), msg);
                    } else {
                        eprintln!("{}", msg);
                    }
                }
                Severity::Warning => {
                    if let Some(code) = entry.error_code() {
                        // config.ui().warn_with_code(code.as_str(), msg)
                        eprintln!("{} {}", code.as_str(), msg);
                    } else {
                        // config.ui().warn(msg)
                        eprintln!("{}", msg);
                    }
                }
            };
        }
    });
}

pub fn edition_from_string(edition_str: &str) -> Result<Edition, serde_json::Error> {
    // Format `edition` to be a valid JSON string.
    let edition = format!("\"{}\"", edition_str);
    serde_json::from_str(&edition)
}
