use assert_fs::TempDir;
use indoc::indoc;

use crate::support::command::Scarb;
use crate::support::project_builder::ProjectBuilder;

#[test]
fn compile_cairo_plugin() {
    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello")
        .version("1.0.0")
        .manifest_extra(r#"[cairo-plugin]"#)
        .build(&t);

    Scarb::quick_snapbox()
        .arg("build")
        .current_dir(&t)
        .assert()
        .failure()
        .stdout_matches(indoc! {r#"
        error: compiling Cairo plugin packages is not possible yet
        "#});
}

#[test]
fn compile_cairo_plugin_with_lib_target() {
    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello")
        .version("1.0.0")
        .manifest_extra(indoc! {r#"
            [lib]
            [cairo-plugin]
        "#})
        .build(&t);

    Scarb::quick_snapbox()
        .arg("build")
        .current_dir(&t)
        .assert()
        .failure()
        .stdout_matches(indoc! {r#"
        error: failed to parse manifest at `[..]/Scarb.toml`

        Caused by:
            target `cairo-plugin` cannot be mixed with other targets
        "#});
}

#[test]
fn compile_cairo_plugin_with_other_target() {
    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello")
        .version("1.0.0")
        .manifest_extra(indoc! {r#"
            [cairo-plugin]
            [[target.starknet-contract]]
        "#})
        .build(&t);

    Scarb::quick_snapbox()
        .arg("build")
        .current_dir(&t)
        .assert()
        .failure()
        .stdout_matches(indoc! {r#"
        error: failed to parse manifest at `[..]/Scarb.toml`

        Caused by:
            target `cairo-plugin` cannot be mixed with other targets
        "#});
}
