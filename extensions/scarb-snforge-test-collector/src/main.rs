use anyhow::{Context, Result};
use clap::Parser;
use fs4::FileExt;
use std::fs::{create_dir_all, File};
use std::io::BufWriter;

use scarb_metadata::MetadataCommand;
use scarb_ui::args::PackagesFilter;

use crate::compilation::compile_tests;
use crate::crate_collection::collect_test_compilation_targets;
use crate::metadata::compilation_unit_for_package;

mod compilation;
mod crate_collection;
mod felt252;
mod metadata;

/// Starknet Foundry private extension for compiling test artifacts.
/// Users should not call it directly.
#[derive(Parser)]
#[command(version)]
struct Args {
    #[command(flatten)]
    packages_filter: PackagesFilter,
}

fn main() -> Result<()> {
    let args = Args::parse();

    let metadata = MetadataCommand::new().inherit_stderr().exec()?;
    let selected_packages_metadata = args.packages_filter.match_many(&metadata)?;

    let target_dir = metadata
        .target_dir
        .clone()
        .unwrap_or_else(|| metadata.workspace.root.join("target"));
    let snforge_target_dir = target_dir.join(&metadata.current_profile).join("snforge");
    create_output_dir::create_output_dir(&target_dir.into_std_path_buf())?;
    create_dir_all(&snforge_target_dir)?;

    for package_metadata in selected_packages_metadata {
        let compilation_unit = compilation_unit_for_package(&metadata, &package_metadata)?;

        let compilation_targets = collect_test_compilation_targets(
            &package_metadata.name,
            package_metadata.version.clone(),
            &package_metadata.root,
            &compilation_unit,
        )?;

        let test_crates = compile_tests(&compilation_targets, &compilation_unit)?;

        // artifact saved to `{target_dir}/{profile_name}/{package_name}.sierra.json`
        let output_path =
            snforge_target_dir.join(format!("{}.snforge_sierra.json", package_metadata.name));
        let output_file = File::options()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&output_path)?;

        output_file
            .lock_exclusive()
            .with_context(|| format!("Couldn't lock the output file = {output_path}"))?;
        let file = BufWriter::new(&output_file);
        serde_json::to_writer(file, &test_crates)
            .with_context(|| format!("Failed to serialize = {output_path}"))?;
        output_file
            .unlock()
            .with_context(|| format!("Couldn't lock the output file = {output_path}"))?;
    }

    Ok(())
}
