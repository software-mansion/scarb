//! Tests for internal code checks.

use std::fs;

use semver::{BuildMetadata, Prerelease, Version};
use toml_edit::Document;

/// Checks that package version in [`Cairo.toml`] is exactly the same as the version of `Cairo`
/// dependency, because this project is tightly coupled with it.
#[test]
#[ignore = "Scarb is not ready to be version synced with Cairo"]
fn project_version_is_bound_to_cairo_version() {
    let cargo_toml: Document = fs::read_to_string("../Cargo.toml")
        .unwrap()
        .parse()
        .unwrap();
    let cargo_lock: Document = fs::read_to_string("../Cargo.lock")
        .unwrap()
        .parse()
        .unwrap();

    let mut package_version: Version = cargo_toml["workspace"]["package"]["version"]
        .as_value()
        .unwrap()
        .as_str()
        .unwrap()
        .parse()
        .unwrap();

    let mut cairo_version: Version = cargo_lock["package"]
        .as_array_of_tables()
        .unwrap()
        .iter()
        .find(|t| t["name"].as_value().unwrap().as_str().unwrap() == "cairo-lang-compiler")
        .unwrap()["version"]
        .as_value()
        .unwrap()
        .as_str()
        .unwrap()
        .parse()
        .unwrap();

    // Allow differences in prerelease and build metadata
    package_version.pre = Prerelease::EMPTY;
    package_version.build = BuildMetadata::EMPTY;
    cairo_version.pre = Prerelease::EMPTY;
    cairo_version.build = BuildMetadata::EMPTY;

    assert_eq!(package_version, cairo_version);
}
