use std::iter;
use std::sync::Arc;

use anyhow::{bail, Result};
use cairo_lang_compiler::db::RootDatabase;
use cairo_lang_compiler::diagnostics::DiagnosticsReporter;
use cairo_lang_compiler::project::{ProjectConfig, ProjectConfigContent};
use cairo_lang_filesystem::cfg::{Cfg, CfgSet};
use cairo_lang_filesystem::db::FilesGroup;
use cairo_lang_filesystem::ids::{CrateLongId, Directory};
use cairo_lang_starknet::inline_macros::selector::SelectorMacro;
use cairo_lang_starknet::plugin::StarkNetPlugin;
use cairo_lang_test_runner::plugin::TestPlugin;
use cairo_lang_test_runner::TestRunner;
use clap::Parser;

use scarb_metadata::{CompilationUnitMetadata, Metadata, MetadataCommand, PackageId};
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

    let starknet_package_id = metadata
        .packages
        .iter()
        .find(|p| p.name == "starknet" && p.source.repr == "std")
        .map(|p| p.id.clone());

    for package in args.packages_filter.match_many(&metadata)? {
        println!("testing {} ...", package.name);

        let Some(unit) = find_testable_compilation_unit(&metadata, &package.id) else {
            println!("warn: package `{}` has no testable targets", package.name);
            continue;
        };

        // Enable the Starknet test plugin if the package depends on the `starknet` package.
        let starknet = if let Some(starknet_package_id) = &starknet_package_id {
            unit.cairo_plugins
                .iter()
                .any(|p| p.package == *starknet_package_id)
        } else {
            false
        };

        let db = build_root_database(unit, starknet)?;

        let main_crate_ids = unit
            .components
            .iter()
            .map(|component| db.intern_crate(CrateLongId::Real(component.name.clone().into())))
            .collect();

        let test_crate_ids = vec![db.intern_crate(CrateLongId::Real(package.name.clone().into()))];

        if DiagnosticsReporter::stderr().check(&db) {
            bail!("could not compile `{}` due to previous error", package.name);
        }

        let runner = TestRunner {
            db,
            main_crate_ids,
            test_crate_ids,
            filter: args.filter.clone(),
            include_ignored: args.include_ignored,
            ignored: args.ignored,
            starknet,
        };
        runner.run()?;

        println!();
    }

    Ok(())
}

fn find_testable_compilation_unit<'a>(
    metadata: &'a Metadata,
    package_id: &PackageId,
) -> Option<&'a CompilationUnitMetadata> {
    metadata
        .compilation_units
        .iter()
        .filter(|unit| unit.package == *package_id)
        .min_by_key(|unit| match unit.target.name.as_str() {
            name @ "lib" => (0, name),
            name => (1, name),
        })
}

fn build_root_database(unit: &CompilationUnitMetadata, starknet: bool) -> Result<RootDatabase> {
    let mut b = RootDatabase::builder();

    b.with_project_config(ProjectConfig {
        base_path: unit.target.source_root().into(),
        corelib: unit
            .components
            .iter()
            .find(|c| c.name == "core")
            .map(|c| Directory::Real(c.source_root().into())),
        content: ProjectConfigContent {
            crate_roots: unit
                .components
                .iter()
                .filter(|c| c.name != "core")
                .map(|c| (c.name.clone().into(), c.source_root().into()))
                .collect(),
        },
    });

    b.with_cfg(
        unit.cfg
            .iter()
            .map(|cfg| {
                serde_json::to_value(cfg)
                    .and_then(serde_json::from_value)
                    .expect("Cairo's `Cfg` must serialize identically as Scarb Metadata's `Cfg`.")
            })
            .chain(iter::once(Cfg::name("test")))
            .collect::<CfgSet>(),
    );

    b.with_macro_plugin(Arc::new(TestPlugin::default()));

    if starknet {
        b.with_macro_plugin(Arc::new(StarkNetPlugin::default()));
        b.with_inline_macro_plugin(SelectorMacro::NAME, Arc::new(SelectorMacro));
    }

    b.build()
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
