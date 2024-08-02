use anyhow::{Context, Result};
use clap::Parser;
use scarb_doc::docs_generation::markdown::MarkdownContent;
use scarb_doc::metadata::get_target_dir;

use scarb_metadata::MetadataCommand;
use scarb_ui::args::PackagesFilter;

use scarb_doc::generate_packages_information;
use scarb_doc::versioned_json_output::VersionedJsonOutput;

use scarb::ops::FeaturesOpts;
use scarb_ui::args::FeaturesSpec;

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

    /// Specify features to enable.
    #[command(flatten)]
    pub features: FeaturesSpec,
}

fn main_inner() -> Result<()> {
    let args = Args::parse();

    let metadata = MetadataCommand::new()
        .inherit_stderr()
        .exec()
        .context("metadata command failed")?;
    let metadata_for_packages = args.packages_filter.match_many(&metadata)?;
    let output_dir = get_target_dir(&metadata).join(OUTPUT_DIR);
    let features_opt: FeaturesOpts = args.features.try_into()?;

    let packages_information =
        generate_packages_information(&metadata, &metadata_for_packages, features_opt)?;

    match args.output_format {
        OutputFormat::Json => {
            VersionedJsonOutput::new(packages_information)
                .save_to_file(&output_dir)
                .context("failed to write output of scarb doc to a file")?;
        }
        OutputFormat::Markdown => {
            for pkg_information in packages_information {
                let pkg_output_dir = output_dir.join(&pkg_information.metadata.name);

                MarkdownContent::from_crate(&pkg_information)
                    .save(&pkg_output_dir)
                    .with_context(|| {
                        format!(
                            "failed to save docs for package {}",
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
