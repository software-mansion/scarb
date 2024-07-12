use anyhow::{Context, Result};
use clap::Parser;
use scarb_doc::docs_generation::markdown::MarkdownContent;
use std::fs;

use scarb_metadata::MetadataCommand;
use scarb_ui::args::PackagesFilter;

use scarb_doc::generate_packages_information;
use scarb_doc::metadata::get_target_dir;
use scarb_doc::versioned_json_output::VersionedJsonOutput;

const OUTPUT_DIR: &str = "doc";

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
    let packages_metadata = args.packages_filter.match_many(&metadata)?;

    let packages_information = generate_packages_information(&metadata, &packages_metadata);

    let output_dir = get_target_dir(&metadata).join(OUTPUT_DIR);

    fs::create_dir_all(&output_dir).context("failed to create output directory for scarb doc")?;

    match args.output_format {
        OutputFormat::Json => {
            let versioned_json_output = VersionedJsonOutput::new(packages_information);
            let output = serde_json::to_string_pretty(&versioned_json_output)
                .expect("failed to serialize information about crates")
                + "\n";
            let output_path = output_dir.join("output.json");

            fs::write(output_path, output)
                .context("failed to write output of scarb doc to a file")?;
        }
        OutputFormat::Markdown => {
            for pkg_information in packages_information {
                let markdown_content = MarkdownContent::from_crate(&pkg_information);
                let pkg_output_dir = output_dir.join(&pkg_information.metadata.name);

                markdown_content.save(&pkg_output_dir).with_context(|| {
                    format!(
                        "Failed to save docs for package {}",
                        pkg_information.metadata.name
                    )
                })?;
            }
        }
    }

    Ok(())
}

fn main() {
    match main_inner() {
        Ok(()) => std::process::exit(0),
        Err(error) => {
            scarb_ui::Ui::new(scarb_ui::Verbosity::Normal, scarb_ui::OutputFormat::Text)
                .error(format!("{error:#}"));
            std::process::exit(1);
        }
    }
}
