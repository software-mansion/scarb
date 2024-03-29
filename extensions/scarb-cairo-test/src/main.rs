use std::{env, fs};

use anyhow::{Context, Result};
use cairo_lang_test_plugin::TestCompilation;
use cairo_lang_test_runner::{CompiledTestRunner, RunProfilerConfig, TestRunConfig};
use clap::Parser;

use scarb_metadata::{
    Metadata, MetadataCommand, PackageId, PackageMetadata, ScarbCommand, TargetMetadata,
};
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

    /// Whether to print resource usage after each test.
    #[arg(long, default_value_t = false)]
    print_resource_usage: bool,

    /// Comma separated list of features to activate
    #[arg(short = 'F', long, default_value = "")]
    pub features: String,

    /// Activate all available features
    #[arg(long, default_value_t = false)]
    pub all_features: bool,

    /// Do not activate the `default` feature
    #[arg(long, default_value_t = false)]
    pub no_default_features: bool,
}

fn main() -> Result<()> {
    let args: Args = Args::parse();

    let metadata = MetadataCommand::new().inherit_stderr().exec()?;

    check_scarb_version(&metadata);

    let matched = args.packages_filter.match_many(&metadata)?;
    let filter = PackagesFilter::generate_for::<Metadata>(matched.iter());

    // TODO: refactor
    let features_env = if args.features.is_empty() {
        vec![]
    } else {
        vec![("SCARB_FEATURES", args.features.clone())]
    };

    ScarbCommand::new()
        .arg("build")
        .arg("--test")
        .envs(features_env)
        .env(
            "SCARB_NO_DEFAULT_FEATURES",
            args.no_default_features.to_string(),
        )
        .env("SCARB_ALL_FEATURES", args.all_features.to_string())
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

        for target in find_testable_targets(&package) {
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
                run_profiler: RunProfilerConfig::None,
                gas_enabled: is_gas_enabled(&metadata, &package.id, target),
                print_resource_usage: args.print_resource_usage,
            };
            let runner = CompiledTestRunner::new(test_compilation, config);
            runner.run(None)?;
            println!();
        }
    }

    Ok(())
}

fn is_gas_enabled(metadata: &Metadata, package_id: &PackageId, target: &TargetMetadata) -> bool {
    metadata
        .compilation_units
        .iter()
        .find(|cu| {
            cu.package == *package_id && cu.target.kind == "test" && cu.target.name == target.name
        })
        .map(|cu| cu.compiler_config.clone())
        .and_then(|c| {
            c.as_object()
                .and_then(|c| c.get("enable_gas").and_then(|x| x.as_bool()))
        })
        // Defaults to true, meaning gas enabled - relies on cli config then.
        .unwrap_or(true)
}

fn find_testable_targets(package: &PackageMetadata) -> Vec<&TargetMetadata> {
    package
        .targets
        .iter()
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
