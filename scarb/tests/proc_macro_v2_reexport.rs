use assert_fs::TempDir;
use assert_fs::prelude::PathChild;
use indoc::indoc;
use scarb_metadata::Metadata;
use scarb_test_support::cairo_plugin_project_builder::CairoPluginProjectBuilder;
use scarb_test_support::command::{CommandExt, Scarb};
use scarb_test_support::project_builder::ProjectBuilder;

#[test]
fn cairo_plugin_re_export_simple() {
    let t = TempDir::new().unwrap();
    CairoPluginProjectBuilder::default()
        .name("world")
        .build(&t.child("world"));
    ProjectBuilder::start()
        .name("beautiful")
        .dep("world", t.child("world"))
        .manifest_package_extra(indoc! {r#"
            re-export-cairo-plugins = ["world"]
        "#})
        .build(&t.child("beautiful"));
    ProjectBuilder::start()
        .name("hello")
        .dep("beautiful", t.child("beautiful"))
        .dep("world", t.child("world"))
        .build(&t.child("hello"));

    let meta = Scarb::quick_snapbox()
        .arg("--json")
        .arg("metadata")
        .arg("--format-version")
        .arg("1")
        .current_dir(t.child("hello"))
        .stdout_json::<Metadata>();
    let hello_cu = meta
        .compilation_units
        .iter()
        .find(|cu| cu.target.name == "hello" && cu.target.kind == "lib")
        .unwrap();

    assert_eq!(
        hello_cu.cairo_plugins.len(),
        1,
        "CU should contain exactly one plugin"
    );
    assert!(
        hello_cu
            .cairo_plugins
            .first()
            .unwrap()
            .package
            .to_string()
            .starts_with("world 1.0.0"),
        "plugin world not found in cu plugins"
    );
    assert_eq!(
        hello_cu.components.len(),
        3,
        "CU should contain exactly three components"
    );
    let beautiful_component = hello_cu
        .components
        .iter()
        .find(|c| c.name == "beautiful")
        .unwrap();
    assert_eq!(
        beautiful_component.dependencies.clone().unwrap().len(),
        3,
        "beautiful component should have 3 dependencies"
    );

    Scarb::quick_snapbox()
        .arg("build")
        // Disable output from Cargo.
        .env("CARGO_TERM_QUIET", "true")
        .current_dir(t.child("hello"))
        .assert()
        .failure()
        .stdout_matches(indoc! {r#"
           [..]Compiling world v1.0.0 [..]
           [..]Compiling hello v1.0.0 [..]
           error: duplicate expansions defined for procedural macros: some (world v1.0.0 ([..]Scarb.toml) and world v1.0.0 ([..]Scarb.toml))
        "#});
}
