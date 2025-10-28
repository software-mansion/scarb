use anyhow::{Result, ensure};
use camino::Utf8PathBuf;
use clap::Parser;
use mimalloc::MiMalloc;
use scarb_doc::diagnostics::print_diagnostics;
use scarb_doc::docs_generation::markdown::{MarkdownContent, WorkspaceMarkdownBuilder};
use scarb_doc::errors::{MetadataCommandError, PackagesSerializationError};
use scarb_doc::metadata::get_target_dir;
use scarb_doc::versioned_json_output::VersionedJsonOutput;
use scarb_doc::{PackageInformation, generate_package_context, generate_package_information};
use scarb_extensions_cli::doc::{Args, OutputFormat};
use scarb_metadata::{MetadataCommand, ScarbCommand};
use scarb_ui::Ui;
use scarb_ui::args::ToEnvVars;
use scarb_ui::components::Status;
use serde_json::Value;
use std::process::ExitCode;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

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
    let workspace_root = metadata.workspace.root.clone();
    if matches!(args.output_format, OutputFormat::Markdown) & args.packages_filter.get_workspace() {
        let mut builder = WorkspaceMarkdownBuilder::default();
        for pm in &metadata_for_packages {
            let ctx = generate_package_context(&metadata, pm, args.document_private_items)?;
            let package_info = generate_package_information(&ctx, ui.clone())?;
            print_diagnostics(&ui);
            builder.add_package(&package_info)?;
        }
        let content = builder.build()?;
        output_markdown(
            content,
            None,
            &output_dir,
            args.build,
            &workspace_root,
            ui.clone(),
        )?;
    } else {
        let mut output = match args.output_format {
            OutputFormat::Json => OutputEmit::for_json(output_dir, workspace_root, ui.clone()),
            OutputFormat::Markdown => {
                OutputEmit::for_markdown(output_dir, workspace_root, args.build, ui.clone())
            }
        };
        for pm in &metadata_for_packages {
            let ctx = generate_package_context(&metadata, pm, args.document_private_items)?;
            let info = generate_package_information(&ctx, ui.clone())?;
            print_diagnostics(&ui);
            output.write(info)?;
        }
        output.flush()?;
    }
    Ok(())
}

pub enum OutputEmit {
    Markdown {
        output_dir: Utf8PathBuf,
        ui: Ui,
        build: bool,
        workspace_root: Utf8PathBuf,
    },
    Json {
        output_dir: Utf8PathBuf,
        ui: Ui,
        workspace_root: Utf8PathBuf,
        packages: Vec<Value>,
    },
}

impl OutputEmit {
    pub fn for_markdown(
        output_dir: Utf8PathBuf,
        workspace_root: Utf8PathBuf,
        build: bool,
        ui: Ui,
    ) -> Self {
        OutputEmit::Markdown {
            output_dir,
            ui,
            build,
            workspace_root,
        }
    }

    pub fn for_json(output_dir: Utf8PathBuf, workspace_root: Utf8PathBuf, ui: Ui) -> Self {
        OutputEmit::Json {
            output_dir,
            ui,
            workspace_root,
            packages: vec![],
        }
    }

    pub fn write(&mut self, package: PackageInformation) -> Result<()> {
        match self {
            OutputEmit::Markdown {
                output_dir,
                build,
                workspace_root,
                ui,
            } => {
                let content = MarkdownContent::from_crate(&package)?;
                output_markdown(
                    content,
                    Some(package.metadata.name),
                    output_dir,
                    *build,
                    workspace_root,
                    ui.clone(),
                )?;
            }
            OutputEmit::Json { packages, .. } => {
                packages.push(
                    serde_json::to_value(&package).map_err(PackagesSerializationError::from)?,
                );
            }
        };
        Ok(())
    }

    pub fn flush(self) -> Result<()> {
        match self {
            OutputEmit::Markdown { .. } => {
                // No need to do anything.
            }
            OutputEmit::Json {
                packages,
                output_dir,
                workspace_root,
                ui,
            } => {
                VersionedJsonOutput::new(packages)
                    .save_to_file(&output_dir, JSON_OUTPUT_FILENAME)?;

                let output_path = output_dir
                    .join(JSON_OUTPUT_FILENAME)
                    .strip_prefix(&workspace_root)
                    .unwrap_or(&output_dir)
                    .to_string();
                ui.print(Status::new("Saving output to:", &output_path));
            }
        };
        Ok(())
    }
}

pub fn output_markdown(
    content: MarkdownContent,
    package_name: Option<String>,
    output_dir: &Utf8PathBuf,
    build: bool,
    workspace_root: &Utf8PathBuf,
    ui: Ui,
) -> Result<()> {
    let output_dir = if let Some(package_name) = package_name {
        output_dir.join(package_name)
    } else {
        output_dir.clone()
    };
    content.save(&output_dir)?;

    let output_path = output_dir
        .strip_prefix(workspace_root)
        .unwrap_or(&output_dir)
        .to_string();
    ui.print(Status::new("Saving output to:", &output_path));
    if build {
        let build_output_dir = output_dir.join("book");
        ScarbCommand::new()
            .arg("mdbook")
            .arg("--input")
            .arg(output_dir.clone())
            .arg("--output")
            .arg(build_output_dir.clone())
            .env("SCARB_UI_VERBOSITY", ui.verbosity().to_string())
            .run()?;
        let build_output_path = build_output_dir
            .strip_prefix(workspace_root)
            .unwrap_or(&build_output_dir)
            .to_string();
        ui.print(Status::new("Saving build output to:", &build_output_path));
        ui.print(format!(
            "\nRun the following to see the results: \n`mdbook serve {output_path}`\
                         \n\nOr open the following in your browser: \n`{workspace_root}/{output_path}/book/index.html`",
        ));
    } else {
        ui.print(format!(
            "\nRun the following to see the results: \n`mdbook serve {output_path}`\n(you will need to have mdbook installed)\
                        \n\nOr build html docs by running `scarb doc --build`",
        ));
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
