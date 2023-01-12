//! Tests for internal code checks.

use std::fs;

use toml_edit::Document;

/// Checks that package version in [`Cairo.toml`] is exactly the same as the version of `Cairo`
/// dependency, because this project is tightly coupled with it.
#[test]
fn project_version_is_bound_to_cairo_version() {
    let cargo_toml: Document = fs::read_to_string("../Cargo.toml")
        .unwrap()
        .parse()
        .unwrap();
    let cargo_lock: Document = fs::read_to_string("../Cargo.lock")
        .unwrap()
        .parse()
        .unwrap();

    let package_version = cargo_toml["workspace"]["package"]["version"]
        .as_value()
        .unwrap()
        .as_str()
        .unwrap();

    let cairo_version = cargo_lock["package"]
        .as_array_of_tables()
        .unwrap()
        .iter()
        .find(|t| t["name"].as_value().unwrap().as_str().unwrap() == "cairo-lang-compiler")
        .unwrap()["version"]
        .as_value()
        .unwrap()
        .as_str()
        .unwrap();

    assert_eq!(package_version, cairo_version);
}
