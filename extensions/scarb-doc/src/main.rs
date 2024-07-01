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
}

fn main() -> Result<()> {
    let args = Args::parse();

    let metadata = MetadataCommand::new().inherit_stderr().exec()?;
    let metadata_for_packages = args.packages_filter.match_many(&metadata)?;

    for package_metadata in metadata_for_packages {
        let project_config = get_project_config(&metadata, &package_metadata);
        let crate_ =
            generate_language_elements_tree_for_package(package_metadata.name, project_config)?;

        println!("{crate_:?}");
    }

    Ok(())
}
