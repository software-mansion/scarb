use anyhow::{Context, Result};
use clap::Parser;
use scarb_doc::docs_generation::markdown::MarkdownContent;
use scarb_doc::errors::MetadataCommandError;
use scarb_doc::metadata::get_target_dir;

use scarb_metadata::MetadataCommand;
use scarb_ui::args::{PackagesFilter, ToEnvVars};

use scarb_doc::generate_packages_information;
use scarb_doc::versioned_json_output::VersionedJsonOutput;

use scarb_ui::args::FeaturesSpec;

const OUTPUT_DIR: &str = "doc";

#[derive(Default, Debug, Clone, clap::ValueEnum)]
enum OutputFormat {
    /// Generates documentation in Markdown format.
    /// Generated files are fully compatible with mdBook. For more information visit https://rust-lang.github.io/mdBook.
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

    /// Specifies features to enable.
    #[command(flatten)]
    pub features: FeaturesSpec,

    /// Generates documentation also for private items.
    #[arg(long, default_value_t = false)]
    document_private_items: bool,
}

fn main_inner() -> Result<()> {
    let args = Args::parse();

    let metadata = MetadataCommand::new()
        .inherit_stderr()
        .envs(args.features.to_env_vars())
        .exec()
        .map_err(MetadataCommandError::from)?;
    let metadata_for_packages = args.packages_filter.match_many(&metadata)?;
    let output_dir = get_target_dir(&metadata).join(OUTPUT_DIR);

    let packages_information_result = generate_packages_information(
        &metadata,
        &metadata_for_packages,
        args.document_private_items,
    );

    let packages_information = packages_information_result?;

    match args.output_format {
        OutputFormat::Json => {
            VersionedJsonOutput::new(packages_information).save_to_file(&output_dir)?
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
