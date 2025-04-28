use crate::args::{Args, OutputFormat};
use crate::db::ScarbDocDatabase;
use crate::docs_generation::markdown::MarkdownContent;
use crate::errors::MetadataCommandError;
use crate::metadata::compilation::{
    crates_with_starknet, get_project_config, get_relevant_compilation_unit,
};
use crate::metadata::get_target_dir;
use crate::versioned_json_output::VersionedJsonOutput;
use anyhow::{Context, Result, ensure};
use cairo_lang_compiler::diagnostics::DiagnosticsReporter;
use cairo_lang_diagnostics::{FormattedDiagnosticEntry, Severity};
use cairo_lang_filesystem::{
    db::{Edition, FilesGroup},
    ids::{CrateId, CrateLongId},
};
use errors::DiagnosticError;
use itertools::Itertools;
use scarb_metadata::{
    CompilationUnitMetadata, Metadata, MetadataCommand, PackageMetadata, ScarbCommand,
};
use scarb_ui::Ui;
use scarb_ui::args::ToEnvVars;
use scarb_ui::components::Status;
use serde::Serialize;
use smol_str::ToSmolStr;
use types::Crate;

pub mod args;
pub mod db;
pub mod docs_generation;
pub mod errors;
pub mod location_links;
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

const OUTPUT_DIR: &str = "doc";
const JSON_OUTPUT_FILENAME: &str = "output.json";

fn main_inner(args: Args, ui: Ui) -> Result<()> {
    ensure!(
        !args.build || matches!(args.output_format, OutputFormat::Markdown),
        "`--build` is only supported for Markdown output format"
    );
    let metadata = MetadataCommand::new()
        .inherit_stderr()
        .envs(args.features.to_env_vars())
        .exec()
        .map_err(MetadataCommandError::from)?;
    let metadata_for_packages = args.packages_filter.match_many(&metadata)?;
    let output_dir = get_target_dir(&metadata).join(OUTPUT_DIR);

    let packages_information = generate_packages_information(
        &metadata,
        &metadata_for_packages,
        args.document_private_items,
        ui.clone(),
    )?;

    match args.output_format {
        OutputFormat::Json => {
            VersionedJsonOutput::new(packages_information)
                .save_to_file(&output_dir, JSON_OUTPUT_FILENAME)?;

            let output_path = output_dir
                .join(JSON_OUTPUT_FILENAME)
                .strip_prefix(&metadata.workspace.root)
                .unwrap_or(&output_dir)
                .to_string();
            ui.print(Status::new("Saving output to:", &output_path));
        }
        OutputFormat::Markdown => {
            for pkg_information in packages_information {
                let pkg_output_dir = output_dir.join(&pkg_information.metadata.name);

                MarkdownContent::from_crate(&pkg_information)?
                    .save(&pkg_output_dir)
                    .with_context(|| {
                        format!(
                            "failed to save docs for package {}",
                            pkg_information.metadata.name
                        )
                    })?;

                let output_path = pkg_output_dir
                    .strip_prefix(&metadata.workspace.root)
                    .unwrap_or(&pkg_output_dir)
                    .to_string();
                ui.print(Status::new("Saving output to:", &output_path));
                if args.build {
                    let build_output_dir = pkg_output_dir.join("book");
                    ScarbCommand::new()
                        .arg("mdbook")
                        .arg("--input")
                        .arg(pkg_output_dir)
                        .arg("--output")
                        .arg(build_output_dir.clone())
                        .env("SCARB_UI_VERBOSITY", ui.verbosity().to_string())
                        .run()?;
                    let output_path = build_output_dir
                        .strip_prefix(&metadata.workspace.root)
                        .unwrap_or(&build_output_dir)
                        .to_string();
                    ui.print(Status::new("Saving build output to:", &output_path));
                }
            }
        }
    }
    Ok(())
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
        let crates_with_starknet = crates_with_starknet(metadata, compilation_unit_metadata);

        let db = ScarbDocDatabase::new(project_config, crates_with_starknet);

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

        let crate_ =
            Crate::new_with_virtual_modules(&db, main_crate_id, should_document_private_items)
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
