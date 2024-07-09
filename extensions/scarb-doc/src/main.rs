use anyhow::{Context, Result};
use clap::Parser;
use scarb_doc::compilation::get_project_config;
use std::fs;

use scarb_metadata::MetadataCommand;
use scarb_ui::args::PackagesFilter;

use scarb_doc::generate_language_elements_tree_for_package;

#[derive(Default, Debug, Clone, clap::ValueEnum)]
enum OutputFormat {
    /// Generates documentation in Markdown format.
    #[default]
    Markdown,
    /// Saves information collected from packages in JSON format instead of generating
    /// documentation.
    /// This may be useful if you want to generate documentation files by yourself.
    /// The precise output structure is not guaranteed to be stable.
    Json,
}

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[command(flatten)]
    packages_filter: PackagesFilter,

    /// Specifies a format of generated files.
    #[arg(long, value_enum, default_value_t)]
    output_format: OutputFormat,
}

fn main_inner() -> Result<()> {
    let args = Args::parse();

    let metadata = MetadataCommand::new()
        .inherit_stderr()
        .exec()
        .context("metadata command failed")?;
    let metadata_for_packages = args.packages_filter.match_many(&metadata)?;

    let mut json_output = serde_json::Map::new();

    for package_metadata in metadata_for_packages {
        let project_config = get_project_config(&metadata, &package_metadata);
        let crate_ = generate_language_elements_tree_for_package(
            package_metadata.name.clone(),
            project_config,
        );

        json_output.insert(
            package_metadata.name,
            serde_json::to_value(crate_).expect("failed to serialize information about a crate"),
        );
    }

    let output_dir = metadata
        .target_dir
        .unwrap_or_else(|| metadata.workspace.root.join("target"))
        .join("doc");

    fs::create_dir_all(&output_dir).context("failed to create output directory for scarb doc")?;

    match args.output_format {
        OutputFormat::Json => {
            let output = serde_json::to_string_pretty(&json_output)
                .expect("failed to serialize information about crates");
            let output_path = output_dir.join("output.json");

            fs::write(output_path, output)
                .context("failed to write output of scarb doc to a file")?;
        }
        OutputFormat::Markdown => todo!("#1424"),
    }

    Ok(())
}

fn main() {
    match main_inner() {
        Ok(()) => std::process::exit(0),
        Err(error) => {
            scarb_ui::Ui::new(scarb_ui::Verbosity::Normal, scarb_ui::OutputFormat::Text)
                .error(error.to_string());
            std::process::exit(1);
        }
    }
}
