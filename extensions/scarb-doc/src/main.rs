use anyhow::{Context, Result, ensure};
use clap::Parser;
use scarb_doc::diagnostics::print_diagnostics;
use scarb_doc::docs_generation::markdown::MarkdownContent;
use scarb_doc::errors::MetadataCommandError;
use scarb_doc::metadata::get_target_dir;
use scarb_extensions_cli::doc::{Args, OutputFormat};
use scarb_metadata::{MetadataCommand, ScarbCommand};
use scarb_ui::args::ToEnvVars;
use std::process::ExitCode;

use scarb_doc::generate_packages_information;
use scarb_doc::versioned_json_output::VersionedJsonOutput;

use scarb_ui::Ui;
use scarb_ui::components::Status;

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
    print_diagnostics(&ui);

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
                        .arg(pkg_output_dir.clone())
                        .arg("--output")
                        .arg(build_output_dir.clone())
                        .env("SCARB_UI_VERBOSITY", ui.verbosity().to_string())
                        .run()?;
                    let build_output_path = build_output_dir
                        .strip_prefix(&metadata.workspace.root)
                        .unwrap_or(&build_output_dir)
                        .to_string();
                    ui.print(Status::new("Saving build output to:", &build_output_path));
                    ui.print(format!(
                        "\nRun the following to see the results: \n`mdbook serve {output_path}`\
                         \n\nOr open the following in your browser: \n`{pkg_output_dir}/book/index.html`",
                    ));
                } else {
                    ui.print(format!(
                        "\nRun the following to see the results: \n`mdbook serve {output_path}`\n(you will need to have mdbook installed)\
                        \n\nOr build html docs by running `scarb doc --build`",
                    ));
                }
            }
        }
    }
    Ok(())
}

fn main() -> ExitCode {
    let args = Args::parse();
    let ui = Ui::new(args.verbose.clone().into(), scarb_ui::OutputFormat::Text);
    match main_inner(args, ui.clone()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            ui.error(format!("{error:#}"));
            ExitCode::FAILURE
        }
    }
}
