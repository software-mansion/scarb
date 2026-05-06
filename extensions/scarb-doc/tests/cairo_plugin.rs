use assert_fs::TempDir;
use assert_fs::prelude::PathChild;
use indoc::indoc;
use scarb_test_support::cairo_plugin_project_builder::CairoPluginProjectBuilder;
use scarb_test_support::command::Scarb;
use scarb_test_support::project_builder::ProjectBuilder;
use scarb_test_support::workspace_builder::WorkspaceBuilder;

const HELLO_WORLD_CODE: &str = indoc! {r#"
    fn hello() -> u32 {
        42
    }
"#};

#[test]
fn workspace_skips_cairo_plugin_member() {
    let root_dir = TempDir::new().unwrap();
    let lib_dir = root_dir.child("hello_world");
    let plugin_dir = root_dir.child("some_plugin");

    ProjectBuilder::start()
        .name("hello_world")
        .edition("2023_01")
        .lib_cairo(HELLO_WORLD_CODE)
        .build(&lib_dir);

    CairoPluginProjectBuilder::default()
        .name("some_plugin")
        .build(&plugin_dir);

    WorkspaceBuilder::start()
        .add_member("hello_world")
        .add_member("some_plugin")
        .build(&root_dir);

    Scarb::quick_command()
        .arg("doc")
        .args(["--workspace", "--disable-remote-linking", "--no-run"])
        .current_dir(&root_dir)
        .assert()
        .success()
        .stdout_eq(indoc! {r#"
            warn: skipping `some_plugin`, generating docs for cairo plugins is not supported
            Saving output to: target/doc

            Run the following to see the results:[..]
            `mdbook serve target/doc`
            (you will need to have mdbook installed)[..]

            Or build html docs by running `scarb doc --build`
        "#});

    // Docs were generated for the regular package only.
    assert!(
        root_dir
            .path()
            .join("target/doc/src/hello_world.md")
            .exists()
    );
    assert!(
        !root_dir
            .path()
            .join("target/doc/src/some_plugin.md")
            .exists()
    );
}

#[test]
fn standalone_cairo_plugin_package_is_skipped() {
    let t = TempDir::new().unwrap();
    CairoPluginProjectBuilder::default()
        .name("some_plugin")
        .build(&t);

    Scarb::quick_command()
        .arg("doc")
        .args(["--disable-remote-linking", "--no-run"])
        .current_dir(&t)
        .assert()
        .success()
        .stdout_eq(indoc! {r#"
            warn: skipping `some_plugin`, generating docs for cairo plugins is not supported
        "#});

    // No docs directory should be produced.
    assert!(!t.path().join("target/doc").exists());
}
