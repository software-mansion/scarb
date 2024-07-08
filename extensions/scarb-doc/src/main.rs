use anyhow::Result;
use clap::Parser;
use scarb_doc::compilation::get_project_config;

use scarb_metadata::MetadataCommand;
use scarb_ui::args::PackagesFilter;

use scarb_doc::generate_language_elements_tree_for_package;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[command(flatten)]
    packages_filter: PackagesFilter,

    /// Print information collected from packages to stdout in JSON format instead of generating a
    /// documentation.
    /// This feature may be useful if you want to generate documentation files by yourself.
    /// The precise output structure is not guaranteed to be stable.
    #[arg(long)]
    unstable_json_output: bool,
}

fn main() -> Result<()> {
    let args = Args::parse();

    let metadata = MetadataCommand::new().inherit_stderr().exec()?;
    let metadata_for_packages = args.packages_filter.match_many(&metadata)?;

    let mut json_output = serde_json::Map::new();

    for package_metadata in metadata_for_packages {
        let project_config = get_project_config(&metadata, &package_metadata);
        let crate_ = generate_language_elements_tree_for_package(
            package_metadata.name.clone(),
            project_config,
        )?;

        json_output.insert(
            package_metadata.name,
            serde_json::to_value(crate_).expect("Failed to serialize information about a crate"),
        );
    }

    if args.unstable_json_output {
        println!(
            "{}",
            serde_json::to_string_pretty(&json_output)
                .expect("Failed to serialize information about crates")
        );
    }

    Ok(())
}
