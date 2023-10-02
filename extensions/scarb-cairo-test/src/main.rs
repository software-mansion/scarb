use std::{env, fs};

use anyhow::{Context, Result};
use cairo_lang_test_plugin::TestCompilation;
use cairo_lang_test_runner::{CompiledTestRunner, TestRunConfig};
use clap::Parser;

use scarb_metadata::{Metadata, MetadataCommand, PackageId, ScarbCommand, TargetMetadata};
use scarb_ui::args::PackagesFilter;

/// Execute all unit tests of a local package.
#[derive(Parser, Clone, Debug)]
#[command(author, version)]
struct Args {
    #[command(flatten)]
    packages_filter: PackagesFilter,

    /// Run only tests whose name contain FILTER.
    #[arg(short, long, default_value = "")]
    filter: String,

    /// Run ignored and not ignored tests.
    #[arg(long, default_value_t = false)]
    include_ignored: bool,

    /// Run only ignored tests.
    #[arg(long, default_value_t = false)]
    ignored: bool,
}

fn main() -> Result<()> {
    let args: Args = Args::parse();

    let metadata = MetadataCommand::new().inherit_stderr().exec()?;

    check_scarb_version(&metadata);

    let matched = args.packages_filter.match_many(&metadata)?;
    let filter = PackagesFilter::generate_for::<Metadata>(matched.iter());
    ScarbCommand::new()
        .arg("build")
        .arg("--test")
        .env("SCARB_PACKAGES_FILTER", filter.to_env())
        .run()?;

    let profile = env::var("SCARB_PROFILE").unwrap_or("dev".into());
    let default_target_dir = metadata.runtime_manifest.join("target");
    let target_dir = metadata
        .target_dir
        .clone()
        .unwrap_or(default_target_dir)
        .join(profile);

    for package in matched {
        println!("testing {} ...", package.name);

        for target in find_testable_targets(&metadata, &package.id) {
            let file_path = target_dir.join(format!("{}.test.json", target.name.clone()));
            let test_compilation = serde_json::from_str::<TestCompilation>(
                &fs::read_to_string(file_path.clone())
                    .with_context(|| format!("failed to read file: {file_path}"))?,
            )
            .with_context(|| format!("failed to deserialize compiled tests file: {file_path}"))?;

            let config = TestRunConfig {
                filter: args.filter.clone(),
                include_ignored: args.include_ignored,
                ignored: args.ignored,
            };
            let runner = CompiledTestRunner::new(test_compilation, config);
            runner.run()?;
            println!();
        }
    }

    Ok(())
}

fn find_testable_targets(metadata: &Metadata, package_id: &PackageId) -> Vec<TargetMetadata> {
    metadata
        .packages
        .iter()
        .filter(|package| package.id == *package_id)
        .flat_map(|package| package.targets.clone())
        .filter(|target| target.kind == "test")
        .collect()
}

fn check_scarb_version(metadata: &Metadata) {
    let app_version = env!("CARGO_PKG_VERSION").to_string();
    let scarb_version = metadata
        .app_version_info
        .clone()
        .version
        .clone()
        .to_string();
    if app_version != scarb_version {
        println!(
            "warn: the version of cairo-test does not match the version of scarb.\
         cairo-test: `{}`, scarb: `{}`",
            app_version, scarb_version
        );
    }
}
