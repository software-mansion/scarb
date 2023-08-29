use std::fs;

use anyhow::{Context, Result};
use cairo_lang_test_runner::{CompiledTestRunner, TestCompilation, TestRunConfig};
use clap::Parser;

use scarb_metadata::{CompilationUnitMetadata, Metadata, MetadataCommand, PackageId, ScarbCommand};
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

    ScarbCommand::new().arg("build").run()?;

    let metadata = MetadataCommand::new().inherit_stderr().exec()?;

    check_scarb_version(&metadata);

    let default_target_dir = metadata.runtime_manifest.join("target");
    let target_dir = metadata
        .target_dir
        .clone()
        .unwrap_or(default_target_dir)
        .join("dev");

    for package in args.packages_filter.match_many(&metadata)? {
        println!("testing {} ...", package.name);

        for cu in find_testable_compilation_units(&metadata, &package.id) {
            let file_path = target_dir.join(format!("{}.test.json", cu.target.name.clone()));
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

fn find_testable_compilation_units<'a>(
    metadata: &'a Metadata,
    package_id: &PackageId,
) -> Vec<&'a CompilationUnitMetadata> {
    metadata
        .compilation_units
        .iter()
        .filter(|unit| unit.package == *package_id && unit.target.kind == "test")
        .collect::<Vec<&CompilationUnitMetadata>>()
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
