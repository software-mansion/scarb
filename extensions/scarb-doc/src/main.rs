use anyhow::{Context, Result};
use clap::Parser;
use scarb_doc::docs_generation::markdown::MarkdownContent;
use scarb_doc::errors::MetadataCommandError;
use scarb_doc::metadata::get_target_dir;
use std::process::ExitCode;

use scarb_metadata::MetadataCommand;
use scarb_ui::args::{PackagesFilter, ToEnvVars, VerbositySpec};

use scarb_doc::generate_packages_information;
use scarb_doc::versioned_json_output::VersionedJsonOutput;

use scarb_ui::args::FeaturesSpec;
use scarb_ui::Ui;

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

    /// Generates documentation also for private items.
    #[arg(long, default_value_t = false)]
    document_private_items: bool,

    /// Specifies features to enable.
    #[command(flatten)]
    pub features: FeaturesSpec,

    /// Logging verbosity.
    #[command(flatten)]
    pub verbose: VerbositySpec,
}

fn main_inner(args: Args, ui: Ui) -> Result<()> {
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
        ui,
    )?;

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
