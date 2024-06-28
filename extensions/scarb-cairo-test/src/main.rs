use std::collections::HashSet;
use std::{env, fs};

use anyhow::{Context, Result};
use cairo_lang_sierra::program::VersionedProgram;
use cairo_lang_test_plugin::{TestCompilation, TestCompilationMetadata};
use cairo_lang_test_runner::{CompiledTestRunner, RunProfilerConfig, TestRunConfig};
use camino::Utf8PathBuf;
use clap::{Parser, ValueEnum};
use indoc::formatdoc;

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

    /// Choose test kind to run.
    #[arg(short, long)]
    pub test_kind: Option<TestKind>,

    /// Whether to print resource usage after each test.
    #[arg(long, default_value_t = false)]
    print_resource_usage: bool,
}

#[derive(ValueEnum, Clone, Debug, Default)]
pub enum TestKind {
    Unit,
    Integration,
    #[default]
    All,
}

impl TestKind {
    pub fn matches(&self, kind: &str) -> bool {
        match self {
            TestKind::Unit => kind == "unit",
            TestKind::Integration => kind == "integration",
            TestKind::All => true,
        }
    }
}

fn main() -> Result<()> {
    let args: Args = Args::parse();

    let metadata = MetadataCommand::new().inherit_stderr().exec()?;

    check_scarb_version(&metadata);
    check_cairo_test_plugin(&metadata);

    let matched = args.packages_filter.match_many(&metadata)?;
    let filter = PackagesFilter::generate_for::<Metadata>(matched.iter());
    let test_kind = args.test_kind.unwrap_or_default();
    let target_names = matched
        .iter()
        .flat_map(|package| {
            find_testable_targets(package)
                .iter()
                .filter(|target| {
                    test_kind.matches(
                        target
                            .params
                            .get("test-type")
                            .and_then(|v| v.as_str())
                            .unwrap_or_default(),
                    )
                })
                .map(|t| t.name.clone())
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();

    ScarbCommand::new()
        .arg("build")
        .arg("--test")
        .env("SCARB_TARGET_NAMES", target_names.clone().join(","))
        .env("SCARB_PACKAGES_FILTER", filter.to_env())
        .run()?;

    let profile = env::var("SCARB_PROFILE").unwrap_or("dev".into());
    let default_target_dir = metadata.runtime_manifest.join("target");
    let target_dir = metadata
        .target_dir
        .clone()
        .unwrap_or(default_target_dir)
        .join(profile);

    let mut deduplicator = TargetGroupDeduplicator::default();
    for package in matched {
        println!("testing {} ...", package.name);
        for target in find_testable_targets(&package) {
            if !target_names.contains(&target.name) {
                continue;
            }
            let name = target
                .params
                .get("group-id")
                .and_then(|v| v.as_str())
                .map(ToString::to_string)
                .unwrap_or(target.name.clone());
            let already_seen = deduplicator.visit(package.name.clone(), name.clone());
            if already_seen {
                continue;
            }
            let test_compilation = deserialize_test_compilation(&target_dir, name.clone())?;
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

fn deserialize_test_compilation(target_dir: &Utf8PathBuf, name: String) -> Result<TestCompilation> {
    let file_path = target_dir.join(format!("{}.test.json", name));
    let test_comp_metadata = serde_json::from_str::<TestCompilationMetadata>(
        &fs::read_to_string(file_path.clone())
            .with_context(|| format!("failed to read file: {file_path}"))?,
    )
    .with_context(|| format!("failed to deserialize compiled tests metadata file: {file_path}"))?;

    let file_path = target_dir.join(format!("{}.test.sierra.json", name));
    let sierra_program = serde_json::from_str::<VersionedProgram>(
        &fs::read_to_string(file_path.clone())
            .with_context(|| format!("failed to read file: {file_path}"))?,
    )
    .with_context(|| format!("failed to deserialize compiled tests sierra file: {file_path}"))?;

    Ok(TestCompilation {
        sierra_program: sierra_program.into_v1()?,
        metadata: test_comp_metadata,
    })
}

#[derive(Default)]
struct TargetGroupDeduplicator {
    seen: HashSet<(String, String)>,
}

impl TargetGroupDeduplicator {
    /// Returns true if already visited.
    pub fn visit(&mut self, package_name: String, group_name: String) -> bool {
        !self.seen.insert((package_name, group_name))
    }
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

fn check_cairo_test_plugin(metadata: &Metadata) {
    let app_version = env!("CARGO_PKG_VERSION").to_string();
    let warn = || {
        println!(
            "{}",
            formatdoc! {r#"
        warn: `cairo_test` plugin not found
        please add the following snippet to your Scarb.toml manifest:
        ```
        [dev-dependencies]
        cairo_test = "{}"
        ```
        "#, app_version}
        );
    };

    let Some(plugin_pkg) = metadata.packages.iter().find(|pkg| {
        pkg.name == "cairo_test"
            && pkg.targets.iter().any(|t| {
                t.kind == "cairo-plugin"
                    && t.name == "cairo_test"
                    && t.params
                        .get("builtin")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false)
            })
    }) else {
        warn();
        return;
    };

    for cu in &metadata.compilation_units {
        if cu.target.kind != "test" {
            continue;
        }
        if !cu
            .cairo_plugins
            .iter()
            .any(|plugin| plugin.package == plugin_pkg.id)
        {
            warn();
            return;
        }
    }
}
