use assert_fs::TempDir;
use assert_fs::prelude::PathChild;
use indoc::{formatdoc, indoc};
use itertools::Itertools;
use scarb_metadata::Metadata;
use scarb_test_support::command::{CommandExt, Scarb};
use scarb_test_support::gitx;
use scarb_test_support::project_builder::{Dep, DepBuilder, ProjectBuilder};
use scarb_test_support::registry::local::LocalRegistry;
use scarb_test_support::workspace_builder::WorkspaceBuilder;
use std::iter::zip;

#[test]
fn can_only_be_defined_in_root() {
    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("first")
        .manifest_extra(indoc! {r#"
            [patch.scarbs-xyz]
            foo = { path = "bar" }
        "#})
        .build(&t.child("first"));
    WorkspaceBuilder::start().add_member("first").build(&t);
    Scarb::quick_snapbox()
        .arg("fetch")
        .current_dir(&t)
        .assert()
        .failure()
        .stdout_matches(indoc! {r#"
            error: failed to parse manifest at: [..]Scarb.toml
    
            Caused by:
                the `[patch]` section can only be defined in the workspace root manifests
                section found in manifest: `[..]first[..]Scarb.toml`
                workspace root manifest: `[..]Scarb.toml`
        "#});
}

#[test]
fn workspace_root_definition_does_not_conflict_with_root_package() {
    let t = TempDir::new().unwrap();
    let patch = t.child("patch");
    ProjectBuilder::start()
        .name("foo")
        .version("2.0.0")
        .build(&patch);
    ProjectBuilder::start()
        .name("first")
        .build(&t.child("first"));
    WorkspaceBuilder::start()
        .add_member("first")
        .package(ProjectBuilder::start().name("root_pkg"))
        .manifest_extra(formatdoc! {r#"
            [patch.scarbs-xyz]
            foo = {}
        "#, patch.build()})
        .build(&t);
    Scarb::quick_snapbox()
        .arg("fetch")
        .current_dir(&t)
        .assert()
        .success();
}

#[test]
fn patch_scarbs_with_path() {
    let t = TempDir::new().unwrap();
    let patch = t.child("patch");
    ProjectBuilder::start()
        .name("foo")
        .version("2.0.0")
        .build(&patch);
    ProjectBuilder::start()
        .name("first")
        .dep("foo", Dep.version("1.0.0"))
        .build(&t.child("first"));
    ProjectBuilder::start()
        .name("second")
        .dep("third", t.child("third"))
        .build(&t.child("second"));
    ProjectBuilder::start()
        .name("third")
        .dep("foo", Dep.version("3.0.0"))
        .build(&t.child("third"));
    WorkspaceBuilder::start()
        .add_member("first")
        .add_member("second")
        .manifest_extra(formatdoc! {r#"
            [patch.scarbs-xyz]
            foo = {}
        "#, patch.build()})
        .build(&t);
    // Assert no warnings are emitted.
    Scarb::quick_snapbox()
        .current_dir(&t)
        .arg("fetch")
        .assert()
        .success()
        .stdout_eq("");
    let metadata = Scarb::quick_snapbox()
        .arg("--json")
        .arg("metadata")
        .arg("--format-version=1")
        .current_dir(&t)
        .stdout_json::<Metadata>();
    let packages = metadata
        .packages
        .into_iter()
        .map(|p| p.id.to_string())
        .sorted()
        .collect_vec();
    let expected = vec![
        "core 2.11.2 (std)".to_string(),
        "first 1.0.0 (path+file:[..]first[..]Scarb.toml)".to_string(),
        "foo 2.0.0 (path+file:[..]patch[..]Scarb.toml)".to_string(),
        "second 1.0.0 (path+file:[..]second[..]Scarb.toml)".to_string(),
        "third 1.0.0 (path+file:[..]third[..]Scarb.toml)".to_string(),
    ];
    for (expected, real) in zip(&expected, packages) {
        snapbox::assert_matches(expected, real);
    }
}

#[test]
fn patch_scarbs_with_git() {
    let git_dep = gitx::new("dep1", |t| {
        ProjectBuilder::start()
            .name("foo")
            .version("2.0.0")
            .build(&t);
    });
    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("first")
        .dep("foo", Dep.version("1.0.0"))
        .build(&t.child("first"));
    ProjectBuilder::start()
        .name("second")
        .dep("third", t.child("third"))
        .build(&t.child("second"));
    ProjectBuilder::start()
        .name("third")
        .dep("foo", Dep.version("3.0.0"))
        .build(&t.child("third"));
    WorkspaceBuilder::start()
        .add_member("first")
        .add_member("second")
        .manifest_extra(formatdoc! {r#"
            [patch.scarbs-xyz]
            foo = {}
        "#, git_dep.build().to_string()})
        .build(&t);
    // Assert no warnings are emitted.
    Scarb::quick_snapbox()
        .current_dir(&t)
        .arg("fetch")
        .assert()
        .success()
        .stdout_matches(indoc! {r#"
            [..]Updating git repository [..]dep1
        "#});
    let metadata = Scarb::quick_snapbox()
        .arg("--json")
        .arg("metadata")
        .arg("--format-version=1")
        .current_dir(&t)
        .stdout_json::<Metadata>();
    let packages = metadata
        .packages
        .into_iter()
        .map(|p| p.id.to_string())
        .sorted()
        .collect_vec();
    let expected = vec![
        "core 2.11.2 (std)".to_string(),
        "first 1.0.0 (path+file:[..]first[..]Scarb.toml)".to_string(),
        "foo 2.0.0 (git+file:[..])".to_string(),
        "second 1.0.0 (path+file:[..]second[..]Scarb.toml)".to_string(),
        "third 1.0.0 (path+file:[..]third[..]Scarb.toml)".to_string(),
    ];
    for (expected, real) in zip(&expected, packages) {
        snapbox::assert_matches(expected, real);
    }
}

#[test]
fn patch_scarbs_with_path_by_full_url() {
    let t = TempDir::new().unwrap();
    let patch = t.child("patch");
    ProjectBuilder::start()
        .name("foo")
        .version("2.0.0")
        .build(&patch);
    ProjectBuilder::start()
        .name("first")
        .dep("foo", Dep.version("1.0.0"))
        .build(&t.child("first"));
    ProjectBuilder::start()
        .name("second")
        .dep("third", t.child("third"))
        .build(&t.child("second"));
    ProjectBuilder::start()
        .name("third")
        .dep("foo", Dep.version("3.0.0"))
        .build(&t.child("third"));
    WorkspaceBuilder::start()
        .add_member("first")
        .add_member("second")
        .manifest_extra(formatdoc! {r#"
            [patch."https://scarbs.xyz/"]
            foo = {}
        "#, patch.build()})
        .build(&t);
    // Assert no warnings are emitted.
    Scarb::quick_snapbox()
        .current_dir(&t)
        .arg("fetch")
        .assert()
        .success()
        .stdout_eq("");
    let metadata = Scarb::quick_snapbox()
        .arg("--json")
        .arg("metadata")
        .arg("--format-version=1")
        .current_dir(&t)
        .stdout_json::<Metadata>();
    let packages = metadata
        .packages
        .into_iter()
        .map(|p| p.id.to_string())
        .sorted()
        .collect_vec();
    let expected = vec![
        "core 2.11.2 (std)".to_string(),
        "first 1.0.0 (path+file:[..]first[..]Scarb.toml)".to_string(),
        "foo 2.0.0 (path+file:[..]patch[..]Scarb.toml)".to_string(),
        "second 1.0.0 (path+file:[..]second[..]Scarb.toml)".to_string(),
        "third 1.0.0 (path+file:[..]third[..]Scarb.toml)".to_string(),
    ];
    for (expected, real) in zip(&expected, packages) {
        snapbox::assert_matches(expected, real);
    }
}

#[test]
fn patch_not_existing_registry_with_path() {
    let t = TempDir::new().unwrap();
    let patch = t.child("patch");
    ProjectBuilder::start()
        .name("foo")
        .version("2.0.0")
        .build(&patch);
    ProjectBuilder::start()
        .name("first")
        .dep(
            "foo",
            Dep.version("1.0.0")
                .with("registry", "https://this-registry-does-not-exist/"),
        )
        .build(&t.child("first"));
    ProjectBuilder::start()
        .name("second")
        .dep("third", t.child("third"))
        .build(&t.child("second"));
    ProjectBuilder::start()
        .name("third")
        .dep(
            "foo",
            Dep.version("3.0.0")
                .with("registry", "https://this-registry-does-not-exist/"),
        )
        .build(&t.child("third"));
    WorkspaceBuilder::start()
        .add_member("first")
        .add_member("second")
        .manifest_extra(formatdoc! {r#"
            [patch."https://this-registry-does-not-exist/"]
            foo = {}
        "#, patch.build()})
        .build(&t);
    // Assert no warnings are emitted.
    Scarb::quick_snapbox()
        .current_dir(&t)
        .arg("fetch")
        .assert()
        .success()
        .stdout_eq("");
    let metadata = Scarb::quick_snapbox()
        .arg("--json")
        .arg("metadata")
        .arg("--format-version=1")
        .current_dir(&t)
        .stdout_json::<Metadata>();
    let packages = metadata
        .packages
        .into_iter()
        .map(|p| p.id.to_string())
        .sorted()
        .collect_vec();
    let expected = vec![
        "core 2.11.2 (std)".to_string(),
        "first 1.0.0 (path+file:[..]first[..]Scarb.toml)".to_string(),
        "foo 2.0.0 (path+file:[..]patch[..]Scarb.toml)".to_string(),
        "second 1.0.0 (path+file:[..]second[..]Scarb.toml)".to_string(),
        "third 1.0.0 (path+file:[..]third[..]Scarb.toml)".to_string(),
    ];
    for (expected, real) in zip(&expected, packages) {
        snapbox::assert_matches(expected, real);
    }
}

#[test]
fn patch_git_with_path() {
    let git_dep = gitx::new("dep1", |t| {
        ProjectBuilder::start()
            .name("foo")
            .version("2.0.0")
            .build(&t);
    });
    let t = TempDir::new().unwrap();
    let patch = t.child("patch");
    ProjectBuilder::start()
        .name("foo")
        .version("2.0.0")
        .build(&patch);
    ProjectBuilder::start()
        .name("first")
        .dep("foo", &git_dep)
        .build(&t.child("first"));
    ProjectBuilder::start()
        .name("second")
        .dep("third", t.child("third"))
        .build(&t.child("second"));
    ProjectBuilder::start()
        .name("third")
        .dep("foo", &git_dep)
        .build(&t.child("third"));
    WorkspaceBuilder::start()
        .add_member("first")
        .add_member("second")
        .manifest_extra(formatdoc! {r#"
            [patch."{}"]
            foo = {}
        "#, git_dep.url(), patch.build()})
        .build(&t);
    // Assert no warnings are emitted.
    Scarb::quick_snapbox()
        .current_dir(&t)
        .arg("fetch")
        .assert()
        .success()
        .stdout_eq("");
    let metadata = Scarb::quick_snapbox()
        .arg("--json")
        .arg("metadata")
        .arg("--format-version=1")
        .current_dir(&t)
        .stdout_json::<Metadata>();
    let packages = metadata
        .packages
        .into_iter()
        .map(|p| p.id.to_string())
        .sorted()
        .collect_vec();
    let expected = vec![
        "core 2.11.2 (std)".to_string(),
        "first 1.0.0 (path+file:[..]first[..]Scarb.toml)".to_string(),
        "foo 2.0.0 (path+file:[..]patch[..]Scarb.toml)".to_string(),
        "second 1.0.0 (path+file:[..]second[..]Scarb.toml)".to_string(),
        "third 1.0.0 (path+file:[..]third[..]Scarb.toml)".to_string(),
    ];
    for (expected, real) in zip(&expected, packages) {
        snapbox::assert_matches(expected, real);
    }
}

#[test]
fn patch_git_with_registry() {
    let git_dep = gitx::new("dep1", |t| {
        ProjectBuilder::start()
            .name("foo")
            .version("2.0.0")
            .build(&t);
    });
    let mut registry = LocalRegistry::create();
    registry.publish(|t| {
        ProjectBuilder::start()
            .name("foo")
            .version("2.0.0")
            .build(t);
    });
    registry.publish(|t| {
        ProjectBuilder::start()
            .name("foo")
            .version("1.0.0")
            .build(t);
    });
    registry.publish(|t| {
        ProjectBuilder::start()
            .name("foo")
            .version("3.0.0")
            .build(t);
    });

    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("first")
        .dep("foo", &git_dep)
        .build(&t.child("first"));
    ProjectBuilder::start()
        .name("second")
        .dep("third", t.child("third"))
        .build(&t.child("second"));
    ProjectBuilder::start()
        .name("third")
        .dep("foo", &git_dep)
        .build(&t.child("third"));
    WorkspaceBuilder::start()
        .add_member("first")
        .add_member("second")
        .manifest_extra(formatdoc! {r#"
            [patch."{}"]
            foo = {{ version = "2.0.0", registry = "{}" }}
        "#, git_dep.url(), registry.to_string()
        })
        .build(&t);
    // Assert no warnings are emitted.
    Scarb::quick_snapbox()
        .current_dir(&t)
        .arg("fetch")
        .assert()
        .success()
        .stdout_eq("");
    let metadata = Scarb::quick_snapbox()
        .arg("--json")
        .arg("metadata")
        .arg("--format-version=1")
        .current_dir(&t)
        .stdout_json::<Metadata>();
    let packages = metadata
        .packages
        .into_iter()
        .map(|p| p.id.to_string())
        .sorted()
        .collect_vec();
    let expected = vec![
        "core 2.11.2 (std)".to_string(),
        "first 1.0.0 (path+file:[..]first[..]Scarb.toml)".to_string(),
        "foo 2.0.0 (registry+file:[..])".to_string(),
        "second 1.0.0 (path+file:[..]second[..]Scarb.toml)".to_string(),
        "third 1.0.0 (path+file:[..]third[..]Scarb.toml)".to_string(),
    ];
    for (expected, real) in zip(&expected, packages) {
        snapbox::assert_matches(expected, real);
    }
}

#[test]
fn invalid_url() {
    let t = TempDir::new().unwrap();
    let patch = t.child("patch");
    ProjectBuilder::start()
        .name("foo")
        .version("2.0.0")
        .build(&patch);
    ProjectBuilder::start()
        .name("first")
        .build(&t.child("first"));
    WorkspaceBuilder::start()
        .add_member("first")
        .manifest_extra(formatdoc! {r#"
            [patch.scarbs.xyz]
            foo = {}
        "#, patch.build()})
        .build(&t);
    Scarb::quick_snapbox()
        .arg("fetch")
        .current_dir(&t)
        .assert()
        .failure()
        .stdout_eq("error: relative URL without a base\n");
}

#[test]
fn warn_unused_patch() {
    let git_dep = gitx::new("dep1", |t| {
        ProjectBuilder::start()
            .name("foo")
            .version("2.0.0")
            .build(&t);
    });
    let t = TempDir::new().unwrap();
    let patch = t.child("patch");
    ProjectBuilder::start()
        .name("boo")
        .version("2.0.0")
        .build(&patch);
    ProjectBuilder::start()
        .name("first")
        .dep("foo", &git_dep)
        .build(&t.child("first"));
    WorkspaceBuilder::start()
        .add_member("first")
        .manifest_extra(formatdoc! {r#"
            [patch."{}"]
            boo = {}
        "#, git_dep.url(), patch.build()})
        .build(&t);
    let metadata = Scarb::quick_snapbox()
        .arg("--json")
        .arg("metadata")
        .arg("--format-version=1")
        .current_dir(&t)
        .stdout_json::<Metadata>();
    let packages = metadata
        .packages
        .into_iter()
        .map(|p| p.id.to_string())
        .sorted()
        .collect_vec();
    let expected = vec![
        "core 2.11.2 (std)".to_string(),
        "first 1.0.0 (path+file:[..]first[..]Scarb.toml)".to_string(),
        "foo 2.0.0 (git+file:[..])".to_string(),
        "second 1.0.0 (path+file:[..]second[..]Scarb.toml)".to_string(),
        "third 1.0.0 (path+file:[..]third[..]Scarb.toml)".to_string(),
    ];
    for (expected, real) in zip(&expected, packages) {
        snapbox::assert_matches(expected, real);
    }
    Scarb::quick_snapbox()
        .arg("fetch")
        .current_dir(&t)
        .assert()
        .success()
        .stdout_matches(indoc! {r#"
            [..]Updating git repository [..]dep1
            warn: patch `boo` (`[..]Scarb.toml`) for source `file:[..]dep1` has not been used
        "#});
}

#[test]
fn packaging_no_version_dependency_ignores_patches() {
    let t = TempDir::new().unwrap();
    let hello = t.child("hello");
    let path_dep = t.child("path_dep");

    ProjectBuilder::start()
        .name("foo")
        .version("1.0.0")
        .build(&path_dep);

    let git_dep = gitx::new("dep1", |t| {
        ProjectBuilder::start()
            .name("foo")
            .version("2.0.0")
            .build(&t);
    });

    ProjectBuilder::start()
        .name("hello")
        .version("1.0.0")
        .dep("foo", &git_dep)
        .manifest_extra(formatdoc! {r#"
            [patch."{}"]
            foo = {}
        "#, git_dep.url(), path_dep.with("version", "1.0.0").build()})
        .build(&hello);

    Scarb::quick_snapbox()
        .arg("package")
        .arg("--no-metadata")
        .arg("--no-verify")
        .current_dir(&hello)
        .assert()
        .failure()
        .stdout_matches(indoc! {r#"
        [..] Packaging hello v1.0.0 [..]
        error: dependency `foo` does not specify a version requirement
        note: all dependencies must have a version specified when packaging
        note: the `git` specification will be removed from dependency declaration
        "#});
}

#[test]
fn patch_registry_with_registry() {
    let mut registry = LocalRegistry::create();
    registry.publish(|t| {
        ProjectBuilder::start()
            .name("bar")
            .version("1.0.0")
            .build(t);
    });
    registry.publish(|t| {
        ProjectBuilder::start()
            .name("bar")
            .version("2.0.0")
            .build(t);
    });
    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("foo")
        .version("0.1.0")
        .dep("bar", Dep.version("1").registry(&registry))
        .manifest_extra(formatdoc! {r#"
            [patch."{}"]
            bar = {}
        "#, registry.url.clone(), Dep.version("2").registry(&registry).build()})
        .build(&t);
    // Assert no warnings are emitted.
    Scarb::quick_snapbox()
        .arg("fetch")
        .current_dir(&t)
        .assert()
        .success()
        .stdout_eq("");
    let metadata = Scarb::quick_snapbox()
        .arg("--json")
        .arg("metadata")
        .arg("--format-version=1")
        .current_dir(&t)
        .stdout_json::<Metadata>();
    let packages = metadata
        .packages
        .into_iter()
        .map(|p| p.id.to_string())
        .sorted()
        .collect_vec();
    let expected = vec![
        "bar 2.0.0 (registry+file:[..])".to_string(),
        "core 2.11.2 (std)".to_string(),
        "foo 0.1.0 (path+[..]Scarb.toml)".to_string(),
    ];
    for (expected, real) in zip(&expected, packages) {
        snapbox::assert_matches(expected, real);
    }
}
