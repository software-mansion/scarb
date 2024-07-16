#![allow(clippy::items_after_test_module)]

use assert_fs::prelude::*;
use assert_fs::TempDir;
use indoc::{formatdoc, indoc};
use test_case::test_case;

use scarb_test_support::manifest_edit::ManifestEditHarness;
use scarb_test_support::project_builder::ProjectBuilder;

#[test]
fn registry_with_version() {
    ManifestEditHarness::offline()
        .args(["add", "dep@1.0.0"])
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
            dep = "1.0.0"
        "#})
        .run();
}

#[test]
fn registry_with_caret_version_req() {
    ManifestEditHarness::offline()
        .args(["add", "dep@1"])
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
            dep = "1"
        "#})
        .run();
}

#[test]
fn registry_without_version() {
    ManifestEditHarness::offline()
        .args(["add", "dep"])
        .input(indoc! {r#"
            [package]
            name = "hello"
            version = "1.0.0"
        "#})
        .failure()
        .stdout_matches(indoc! {r#"
            error: please specify package version requirement, for example: dep@1.0.0
        "#})
        .run();
}

#[test]
fn no_dependencies_section() {
    ManifestEditHarness::offline()
        .args(["add", "dep@1.0.0"])
        .input(indoc! {r#"
            [package]
            name = "hello"
            version = "1.0.0"
        "#})
        .output(indoc! {r#"
            [package]
            name = "hello"
            version = "1.0.0"

            [dependencies]
            dep = "1.0.0"
        "#})
        .run();
}

#[test]
fn dry_run() {
    ManifestEditHarness::offline()
        .args(["add", "--dry-run", "dep@1.0.0"])
        .input(indoc! {r#"
            [package]
            name = "hello"
            version = "1.0.0"

            [dependencies]
            bar = "1.0.0"
        "#})
        .stdout_matches(indoc! {r#"
            warn: aborting due to dry run
        "#})
        .run();
}

#[test]
fn path() {
    let t = TempDir::new().unwrap();

    let dep = t.child("dep");
    ProjectBuilder::start()
        .name("dep")
        .version("1.0.0")
        .build(&dep);

    ManifestEditHarness::new()
        .path(t.child("hello"))
        .args(["add", "dep", "--path"])
        .arg(dep.path())
        .input(indoc! {r#"
            [package]
            name = "hello"
            version = "1.0.0"
        "#})
        .output(indoc! {r#"
            [package]
            name = "hello"
            version = "1.0.0"

            [dependencies]
            dep = { path = "../dep" }
        "#})
        .run();
}

#[test]
fn path_version() {
    let t = TempDir::new().unwrap();

    let dep = t.child("dep");
    ProjectBuilder::start()
        .name("dep")
        .version("1.0.0")
        .build(&dep);

    ManifestEditHarness::new()
        .path(t.child("hello"))
        .args(["add", "dep@1.0.0", "--path"])
        .arg(dep.path())
        .input(indoc! {r#"
            [package]
            name = "hello"
            version = "1.0.0"
        "#})
        .output(indoc! {r#"
            [package]
            name = "hello"
            version = "1.0.0"

            [dependencies]
            dep = { version = "1.0.0", path = "../dep" }
        "#})
        .run();
}

#[test]
fn runs_resolver_if_network_is_allowed() {
    let t = TempDir::new().unwrap();

    let dep = t.child("dep");
    ProjectBuilder::start()
        .name("dep")
        .version("2.0.0")
        .build(&dep);

    ManifestEditHarness::new()
        .path(t.child("hello"))
        .args(["add", "dep@1.0.0", "--path"])
        .arg(dep.path())
        .input(indoc! {r#"
            [package]
            name = "hello"
            version = "1.0.0"
        "#})
        .output(indoc! {r#"
            [package]
            name = "hello"
            version = "1.0.0"

            [dependencies]
            dep = { version = "1.0.0", path = "../dep" }
        "#})
        .failure()
        .stdout_matches(indoc! {r#"
            error: cannot get dependencies of `hello@1.0.0`

            Caused by:
                cannot find package `dep ^1.0.0`
        "#})
        .run();
}

#[test]
fn git() {
    ManifestEditHarness::offline()
        .args(["add", "dep", "--git", "https://example.com"])
        .input(indoc! {r#"
            [package]
            name = "hello"
            version = "1.0.0"
        "#})
        .output(indoc! {r#"
            [package]
            name = "hello"
            version = "1.0.0"

            [dependencies]
            dep = { git = "https://example.com/" }
        "#})
        .run();
}

#[test]
fn git_version() {
    ManifestEditHarness::offline()
        .args(["add", "dep@1.0.0", "--git", "https://example.com"])
        .input(indoc! {r#"
            [package]
            name = "hello"
            version = "1.0.0"
        "#})
        .output(indoc! {r#"
            [package]
            name = "hello"
            version = "1.0.0"

            [dependencies]
            dep = { version = "1.0.0", git = "https://example.com/" }
        "#})
        .run();
}

#[test_case("branch")]
#[test_case("tag")]
#[test_case("rev")]
fn git_spec(what: &str) {
    ManifestEditHarness::offline()
        .args(["add", "dep", "--git", "https://example.com"])
        .arg(format!("--{what}"))
        .arg("abcd")
        .input(indoc! {r#"
            [package]
            name = "hello"
            version = "1.0.0"
        "#})
        .output(formatdoc! {r#"
            [package]
            name = "hello"
            version = "1.0.0"

            [dependencies]
            dep = {{ git = "https://example.com/", {what} = "abcd" }}
        "#})
        .run();
}

#[test]
fn overwrite_registry_version() {
    ManifestEditHarness::offline()
        .args(["add", "dep@2.0.0"])
        .input(indoc! {r#"
            [package]
            name = "hello"
            version = "1.0.0"

            [dependencies]
            dep = "1.0.0"
        "#})
        .output(indoc! {r#"
            [package]
            name = "hello"
            version = "1.0.0"

            [dependencies]
            dep = "2.0.0"
        "#})
        .run();
}

#[test]
fn overwrite_registry_version_simplifies() {
    ManifestEditHarness::offline()
        .args(["add", "dep@2.0.0"])
        .input(indoc! {r#"
            [package]
            name = "hello"
            version = "1.0.0"

            [dependencies]
            dep = { version = "1.0.0" }
        "#})
        .output(indoc! {r#"
            [package]
            name = "hello"
            version = "1.0.0"

            [dependencies]
            dep = "2.0.0"
        "#})
        .run();
}

#[test]
fn overwrite_change_source_from_path_to_git() {
    let t = TempDir::new().unwrap();

    let dep = t.child("dep");
    ProjectBuilder::start()
        .name("dep")
        .version("1.0.0")
        .build(&dep);

    ManifestEditHarness::offline()
        .path(t.child("hello"))
        .args([
            "add",
            "dep",
            "--git",
            "https://example.com",
            "--branch",
            "abc",
        ])
        .input(indoc! {r#"
            [package]
            name = "hello"
            version = "1.0.0"

            [dependencies]
            dep = { version = "1.2.3", path = "../dep" }
        "#})
        .output(indoc! {r#"
            [package]
            name = "hello"
            version = "1.0.0"

            [dependencies]
            dep = { version = "1.2.3", git = "https://example.com/", branch = "abc" }
        "#})
        .run();
}

#[test]
fn should_sort_if_already_sorted() {
    ManifestEditHarness::offline()
        .args(["add", "cat@2.0.0"])
        .input(indoc! {r#"
            [package]
            name = "hello"
            version = "1.0.0"

            [dependencies]
            bar = "1.0.0"
            dep = "1.0.0"
            foo = "1.0.0"
        "#})
        .output(indoc! {r#"
            [package]
            name = "hello"
            version = "1.0.0"

            [dependencies]
            bar = "1.0.0"
            cat = "2.0.0"
            dep = "1.0.0"
            foo = "1.0.0"
        "#})
        .run();

    ManifestEditHarness::offline()
        .args(["add", "cat@2.0.0"])
        .input(indoc! {r#"
            [package]
            name = "hello"
            version = "1.0.0"

            [dependencies]
            bar = "1.0.0"
            
            dep = "1.0.0"
            foo = "1.0.0"
        "#})
        .output(indoc! {r#"
            [package]
            name = "hello"
            version = "1.0.0"

            [dependencies]
            bar = "1.0.0"
            cat = "2.0.0"

            dep = "1.0.0"
            foo = "1.0.0"
        "#})
        .run();

    ManifestEditHarness::offline()
        .args(["add", "dog@2.0.0"])
        .input(indoc! {r#"
            [package]
            name = "hello"
            version = "1.0.0"

            [dependencies]
            bar = "1.0.0"
            cat = "2.0.0"
            
            dep = "1.0.0"
            foo = "1.0.0"
        "#})
        .output(indoc! {r#"
            [package]
            name = "hello"
            version = "1.0.0"

            [dependencies]
            bar = "1.0.0"
            cat = "2.0.0"
            
            dep = "1.0.0"
            dog = "2.0.0"
            foo = "1.0.0"
        "#})
        .run();
}

#[test]
fn should_not_sort_if_already_unsorted() {
    ManifestEditHarness::offline()
        .args(["add", "apple@1.0.0"])
        .input(indoc! {r#"
            [package]
            name = "hello"
            version = "1.0.0"

            [dependencies]
            bar = "1.0.0"
            foo = "1.0.0"
            dep = "1.0.0"
        "#})
        .output(indoc! {r#"
            [package]
            name = "hello"
            version = "1.0.0"

            [dependencies]
            bar = "1.0.0"
            foo = "1.0.0"
            dep = "1.0.0"
            apple = "1.0.0"
        "#})
        .run();
}

#[test]
fn add_dev_dep() {
    ManifestEditHarness::offline()
        .args(["add", "--dev", "foo@1.0.0"])
        .input(indoc! {r#"
            [package]
            name = "hello"
            version = "1.0.0"
        "#})
        .output(indoc! {r#"
            [package]
            name = "hello"
            version = "1.0.0"

            [dev-dependencies]
            foo = "1.0.0"
        "#})
        .run();
}

#[test]
fn add_git_dep_with_invalid_url() {
    ManifestEditHarness::offline()
        .args(["add", "dep", "--git", "example.com"])
        .input(indoc! {r#"
            [package]
            name = "hello"
            version = "1.0.0"
        "#})
        .failure()
        .stdout_matches(indoc! {r#"
            error: invalid URL provided: example.com
            help: use an absolute URL to the Git repository


            Caused by:
                relative URL without a base
        "#})
        .run()
}
