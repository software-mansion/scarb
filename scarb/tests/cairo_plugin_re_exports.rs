use assert_fs::prelude::PathChild;
use assert_fs::TempDir;
use indoc::indoc;
use itertools::Itertools;
use scarb_metadata::Metadata;
use scarb_test_support::command::{CommandExt, Scarb};
use scarb_test_support::project_builder::{Dep, DepBuilder, ProjectBuilder};
use scarb_test_support::registry::local::LocalRegistry;

#[test]
fn cairo_plugin_re_export_simple() {
    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("world")
        .manifest_extra(indoc! {r#"
            [cairo-plugin]
            # Stop Scarb from attempting to compile the plugin with Cargo
            builtin = true
        "#})
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
    let beautiful_deps = beautiful_component
        .dependencies
        .clone()
        .unwrap()
        .into_iter()
        .map(|dep| dep.id.to_string())
        .collect_vec();
    let beautiful_deps_packages = meta
        .packages
        .iter()
        .filter(|pkg| beautiful_deps.iter().contains(&pkg.id.to_string()))
        .map(|pkg| pkg.name.to_string())
        .sorted()
        .collect_vec();
    assert_eq!(
        beautiful_deps_packages.len(),
        beautiful_deps.len(),
        "all dependencies should be found in the packages list"
    );
    assert_eq!(
        beautiful_deps_packages,
        vec!["beautiful", "core", "world"],
        "beautiful component invalid dependencies"
    );
    let hello_component = hello_cu
        .components
        .iter()
        .find(|c| c.name == "hello")
        .unwrap();
    assert_eq!(
        hello_component.dependencies.clone().unwrap().len(),
        4,
        "hello component should have 4 dependencies"
    );
    let hello_deps = hello_component
        .dependencies
        .clone()
        .unwrap()
        .into_iter()
        .map(|dep| dep.id.to_string())
        .collect_vec();
    let hello_deps_packages = meta
        .packages
        .iter()
        .filter(|pkg| hello_deps.iter().contains(&pkg.id.to_string()))
        .map(|pkg| pkg.name.to_string())
        .sorted()
        .collect_vec();
    assert_eq!(
        hello_deps_packages.len(),
        hello_deps.len(),
        "all dependencies should be found in the packages list"
    );
    // Note, hello does depend on world.
    assert_eq!(
        hello_deps_packages,
        vec!["beautiful", "core", "hello", "world"],
        "beautiful component invalid dependencies"
    );
}

#[test]
fn can_only_re_export_own_dep() {
    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("world")
        .manifest_extra(indoc! {r#"
            [cairo-plugin]
            # Stop Scarb from attempting to compile the plugin with Cargo
            builtin = true
        "#})
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
        .manifest_package_extra(indoc! {r#"
            re-export-cairo-plugins = ["world"]
        "#})
        .build(&t.child("hello"));
    ProjectBuilder::start()
        .name("foo")
        .dep("hello", t.child("hello"))
        .build(&t.child("foo"));

    Scarb::quick_snapbox()
        .arg("check")
        .current_dir(t.child("foo"))
        .assert()
        .failure()
        .stdout_eq("error: package `hello` cannot re-export cairo plugin `world` which is not a dependency of `hello`\n");
}

#[test]
fn can_use_re_exports_through_registry() {
    let mut registry = LocalRegistry::create();
    registry.publish(|t| {
        ProjectBuilder::start()
            .name("world")
            .version("1.0.0")
            .manifest_extra(indoc! {r#"
                [cairo-plugin]
                # Stop Scarb from attempting to compile the plugin with Cargo
                builtin = true
            "#})
            .build(t);
    });
    let dep = Dep.version("1");
    let dep = dep.registry(&registry);
    registry.publish(|t| {
        ProjectBuilder::start()
            .name("beautiful")
            .version("1.0.0")
            .dep("world", dep)
            .manifest_package_extra(indoc! {r#"
                re-export-cairo-plugins = ["world"]
            "#})
            .build(t);
    });
    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello")
        .dep("beautiful", Dep.version("1").registry(&registry))
        .build(&t);
    let meta = Scarb::quick_snapbox()
        .arg("--json")
        .arg("metadata")
        .arg("--format-version")
        .arg("1")
        .current_dir(&t)
        .stdout_json::<Metadata>();
    let cu = meta
        .compilation_units
        .iter()
        .find(|cu| cu.target.name == "hello" && cu.target.kind == "lib")
        .unwrap();
    let hello_component = cu.components.iter().find(|c| c.name == "hello").unwrap();
    assert_eq!(
        hello_component.dependencies.clone().unwrap().len(),
        4,
        "hello component should have 4 dependencies"
    );
    let hello_deps = hello_component
        .dependencies
        .clone()
        .unwrap()
        .into_iter()
        .map(|dep| dep.id.to_string())
        .collect_vec();
    let hello_deps_packages = meta
        .packages
        .iter()
        .filter(|pkg| hello_deps.iter().contains(&pkg.id.to_string()))
        .map(|pkg| pkg.name.to_string())
        .sorted()
        .collect_vec();
    assert_eq!(
        hello_deps_packages.len(),
        hello_deps.len(),
        "all dependencies should be found in the packages list"
    );
    // Note, hello does depend on world.
    assert_eq!(
        hello_deps_packages,
        vec!["beautiful", "core", "hello", "world"],
        "beautiful component invalid dependencies"
    );
}
