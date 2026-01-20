#![deny(clippy::dbg_macro)]
#![deny(clippy::disallowed_methods)]

use crate::db::ScarbDocDatabase;
use crate::metadata::compilation::{
    crates_with_starknet, get_project_config, get_relevant_compilation_unit,
};
use std::path::PathBuf;

use crate::types::crate_type::Crate;

use crate::linking::{RemoteDocLinkingData, resolve_remote_linking_data};
use anyhow::Result;
use cairo_lang_compiler::diagnostics::DiagnosticsReporter;
use cairo_lang_diagnostics::{FormattedDiagnosticEntry, Severity};
use cairo_lang_filesystem::ids::SmolStrId;
use cairo_lang_filesystem::{
    db::{Edition, FilesGroup},
    ids::{CrateId, CrateLongId},
};
use cairo_lang_utils::Intern;
use camino::Utf8PathBuf;
use errors::DiagnosticError;
use itertools::Itertools;
use scarb_metadata::{
    CompilationUnitComponentMetadata, CompilationUnitMetadata, Metadata, PackageMetadata,
};
use scarb_ui::Ui;
use serde::Serialize;

pub mod attributes;
pub mod db;
pub mod diagnostics;
pub mod docs_generation;
pub mod errors;
pub mod linking;
pub mod location_links;
pub mod metadata;
pub mod types;
pub mod versioned_json_output;

#[derive(Serialize, Clone)]
pub struct PackageInformation<'db> {
    pub crate_: Crate<'db>,
    pub metadata: AdditionalMetadata,
    #[serde(skip)]
    pub remote_linking_data: RemoteDocLinkingData,
}

#[derive(Serialize, Clone)]
pub struct AdditionalMetadata {
    pub name: String,
    pub authors: Option<Vec<String>>,
    #[serde(skip)]
    pub repository: Option<String>,
}

pub struct PackageContext {
    pub db: ScarbDocDatabase,
    pub should_document_private_items: bool,
    pub metadata: AdditionalMetadata,
    package_compilation_unit: Option<CompilationUnitMetadata>,
    main_component: CompilationUnitComponentMetadata,
}

pub fn generate_package_context(
    metadata: &Metadata,
    package_metadata: &PackageMetadata,
    document_private_items: bool,
) -> Result<PackageContext> {
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
    let project_config = get_project_config(metadata, package_metadata, compilation_unit_metadata)?;
    let crates_with_starknet = crates_with_starknet(metadata, compilation_unit_metadata);

    let db = ScarbDocDatabase::new(project_config, crates_with_starknet);

    let main_component = compilation_unit_metadata
        .components
        .iter()
        .find(|component| component.package == compilation_unit_metadata.package)
        .expect("main component is guaranteed to exist in compilation unit");

    let package_compilation_unit = metadata
        .compilation_units
        .iter()
        .find(|unit| unit.package == package_metadata.id)
        .cloned();

    Ok(PackageContext {
        db,
        should_document_private_items,
        package_compilation_unit,
        main_component: main_component.clone(),
        metadata: AdditionalMetadata {
            name: package_metadata.name.clone(),
            authors,
            repository: package_metadata.manifest_metadata.repository.clone(),
        },
    })
}

pub fn generate_package_information<'a>(
    context: &'a PackageContext,
    ui: &Ui,
    workspace_root: &Utf8PathBuf,
    repo_root: &Option<PathBuf>,
    commit_hash: &Option<String>,
    disable_linking: bool,
    remote_base_url: &Option<String>,
) -> Result<PackageInformation<'a>> {
    let db = &context.db;

    let main_crate_id = CrateLongId::Real {
        name: SmolStrId::from(db, context.main_component.name.as_str()),
        discriminator: context
            .main_component
            .discriminator
            .as_ref()
            .map(ToString::to_string),
    }
    .intern(db);

    let mut diagnostics_reporter =
        setup_diagnostics_reporter(db, main_crate_id, &context.package_compilation_unit, ui)
            .skip_lowering_diagnostics();

    let crate_ = Crate::new_with_virtual_modules_and_groups(
        db,
        main_crate_id,
        context.should_document_private_items,
    )
    .map_err(|_| DiagnosticError(context.metadata.name.clone()));

    if crate_.is_err() {
        diagnostics_reporter.ensure(db)?;
    }

    let crate_ = crate_?;

    let remote_linking_data = resolve_remote_linking_data(
        ui,
        workspace_root,
        repo_root,
        commit_hash,
        disable_linking,
        remote_base_url,
        &context.metadata.repository,
    )?;

    Ok(PackageInformation {
        crate_,
        metadata: context.metadata.clone(),
        remote_linking_data,
    })
}

fn setup_diagnostics_reporter<'a>(
    db: &ScarbDocDatabase,
    main_crate_id: CrateId,
    package_compilation_unit: &Option<CompilationUnitMetadata>,
    ui: &'a Ui,
) -> DiagnosticsReporter<'a> {
    let ignore_warnings_crates = db
        .crates()
        .iter()
        .filter(|&&crate_id| crate_id != main_crate_id)
        .map(|c| c.long(db).clone().into_crate_input(db))
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
    let edition = format!("\"{edition_str}\"");
    serde_json::from_str(&edition)
}
