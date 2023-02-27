use indoc::indoc;

use crate::support::manifest_edit::ManifestEditHarness;

#[test]
fn remove_one() {
    ManifestEditHarness::offline()
        .args(["rm", "foo"])
        .input(indoc! {r#"
            [package]
            name = "hello"
            version = "1.0.0"

            [dependencies]
            dep = "1.0.0"
            foo = "1.0.0"
            bar = "1.0.0"
        "#})
        .output(indoc! {r#"
            [package]
            name = "hello"
            version = "1.0.0"

            [dependencies]
            dep = "1.0.0"
            bar = "1.0.0"
        "#})
        .stdout_matches("    Removing foo from dependencies\n")
        .run();
}

#[test]
fn multiple_deps() {
    ManifestEditHarness::offline()
        .args(["remove", "bar", "dep"])
        .input(indoc! {r#"
            [package]
            name = "hello"
            version = "1.0.0"

            [dependencies]
            dep = "1.0.0"
            foo = "1.0.0"
            bar = "1.0.0"
        "#})
        .output(indoc! {r#"
            [package]
            name = "hello"
            version = "1.0.0"

            [dependencies]
            foo = "1.0.0"
        "#})
        .stdout_matches("    Removing bar from dependencies\n    Removing dep from dependencies\n")
        .run();
}

#[test]
fn undefined_dep() {
    ManifestEditHarness::offline()
        .args(["remove", "foo"])
        .input(indoc! {r#"
            [package]
            name = "hello"
            version = "1.0.0"

            [dependencies]
            dep = "1.0.0"
            bar = "1.0.0"
        "#})
        .failure()
        .stdout_matches(indoc! {r#"    Removing foo from dependencies
            error: the dependency `foo` could not be found in `dependencies`
        "#})
        .run();
}

#[test]
fn no_dependencies_section() {
    ManifestEditHarness::offline()
        .args(["rm", "dep"])
        .input(indoc! {r#"
            [package]
            name = "hello"
            version = "1.0.0"
        "#})
        .failure()
        .stdout_matches(indoc! {r#"    Removing dep from dependencies
            error: the dependency `dep` could not be found in `dependencies`
        "#})
        .run();
}

#[test]
fn dry_run() {
    ManifestEditHarness::offline()
        .args(["remove", "--dry-run", "bar"])
        .input(indoc! {r#"
            [package]
            name = "hello"
            version = "1.0.0"

            [dependencies]
            bar = "1.0.0"
        "#})
        .output(indoc! {r#"
            [package]
            name = "hello"
            version = "1.0.0"

            [dependencies]
            bar = "1.0.0"
        "#})
        .stdout_matches(indoc! {r#"    Removing bar from dependencies
            warn: aborting due to dry run
        "#})
        .run();
}
