use assert_fs::TempDir;
use assert_fs::prelude::PathChild;
use indoc::{formatdoc, indoc};
use itertools::Itertools;
use scarb_build_metadata::CAIRO_VERSION;
use scarb_metadata::Metadata;
use scarb_test_support::command::{CommandExt, Scarb};
use scarb_test_support::gitx;
use scarb_test_support::project_builder::{Dep, DepBuilder, ProjectBuilder};
use scarb_test_support::registry::local::LocalRegistry;
use scarb_test_support::workspace_builder::WorkspaceBuilder;
use snapbox::{Assert, Data};
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
    Scarb::quick_command()
        .arg("fetch")
        .current_dir(&t)
        .assert()
        .failure()
        .stdout_eq(indoc! {r#"
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
    Scarb::quick_command()
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
    Scarb::quick_command()
        .current_dir(&t)
        .arg("fetch")
        .assert()
        .success()
        .stdout_eq(Data::from("").raw());
    let metadata = Scarb::quick_command()
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
        "core [..] (std)".to_string(),
        "first 1.0.0 (path+file:[..]first[..]Scarb.toml)".to_string(),
        "foo 2.0.0 (path+file:[..]patch[..]Scarb.toml)".to_string(),
        "second 1.0.0 (path+file:[..]second[..]Scarb.toml)".to_string(),
        "third 1.0.0 (path+file:[..]third[..]Scarb.toml)".to_string(),
    ];
    for (expected, real) in zip(&expected, packages) {
        Assert::new().eq(real, expected);
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
    Scarb::quick_command()
        .current_dir(&t)
        .arg("fetch")
        .assert()
        .success()
        .stdout_eq(indoc! {r#"
            [..]Updating git repository [..]dep1
        "#});
    let metadata = Scarb::quick_command()
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
        "core [..] (std)".to_string(),
        "first 1.0.0 (path+file:[..]first[..]Scarb.toml)".to_string(),
        "foo 2.0.0 (git+file:[..])".to_string(),
        "second 1.0.0 (path+file:[..]second[..]Scarb.toml)".to_string(),
        "third 1.0.0 (path+file:[..]third[..]Scarb.toml)".to_string(),
    ];
    for (expected, real) in zip(&expected, packages) {
        Assert::new().eq(real, expected);
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
    Scarb::quick_command()
        .current_dir(&t)
        .arg("fetch")
        .assert()
        .success()
        .stdout_eq(Data::from("").raw());
    let metadata = Scarb::quick_command()
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
        "core [..] (std)".to_string(),
        "first 1.0.0 (path+file:[..]first[..]Scarb.toml)".to_string(),
        "foo 2.0.0 (path+file:[..]patch[..]Scarb.toml)".to_string(),
        "second 1.0.0 (path+file:[..]second[..]Scarb.toml)".to_string(),
        "third 1.0.0 (path+file:[..]third[..]Scarb.toml)".to_string(),
    ];
    for (expected, real) in zip(&expected, packages) {
        Assert::new().eq(real, expected);
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
    Scarb::quick_command()
        .current_dir(&t)
        .arg("fetch")
        .assert()
        .success()
        .stdout_eq(Data::from("").raw());
    let metadata = Scarb::quick_command()
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
        "core [..] (std)".to_string(),
        "first 1.0.0 (path+file:[..]first[..]Scarb.toml)".to_string(),
        "foo 2.0.0 (path+file:[..]patch[..]Scarb.toml)".to_string(),
        "second 1.0.0 (path+file:[..]second[..]Scarb.toml)".to_string(),
        "third 1.0.0 (path+file:[..]third[..]Scarb.toml)".to_string(),
    ];
    for (expected, real) in zip(&expected, packages) {
        Assert::new().eq(real, expected);
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
    Scarb::quick_command()
        .current_dir(&t)
        .arg("fetch")
        .assert()
        .success()
        .stdout_eq(Data::from("").raw());
    let metadata = Scarb::quick_command()
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
        "core [..] (std)".to_string(),
        "first 1.0.0 (path+file:[..]first[..]Scarb.toml)".to_string(),
        "foo 2.0.0 (path+file:[..]patch[..]Scarb.toml)".to_string(),
        "second 1.0.0 (path+file:[..]second[..]Scarb.toml)".to_string(),
        "third 1.0.0 (path+file:[..]third[..]Scarb.toml)".to_string(),
    ];
    for (expected, real) in zip(&expected, packages) {
        Assert::new().eq(real, expected);
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
    Scarb::quick_command()
        .current_dir(&t)
        .arg("fetch")
        .assert()
        .success()
        .stdout_eq(Data::from("").raw());
    let metadata = Scarb::quick_command()
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
        "core [..] (std)".to_string(),
        "first 1.0.0 (path+file:[..]first[..]Scarb.toml)".to_string(),
        "foo 2.0.0 (registry+file:[..])".to_string(),
        "second 1.0.0 (path+file:[..]second[..]Scarb.toml)".to_string(),
        "third 1.0.0 (path+file:[..]third[..]Scarb.toml)".to_string(),
    ];
    for (expected, real) in zip(&expected, packages) {
        Assert::new().eq(real, expected);
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
            [patch."scarbs.xyz"]
            foo = {}
        "#, patch.build()})
        .build(&t);
    Scarb::quick_command()
        .arg("fetch")
        .current_dir(&t)
        .assert()
        .failure()
        .stdout_eq(indoc! { r#"
            error: failed to parse manifest at: [..]Scarb.toml

            Caused by:
                0: failed to parse `scarbs.xyz` as patch source url
                1: relative URL without a base
        "#});
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
    let metadata = Scarb::quick_command()
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
        "core [..] (std)".to_string(),
        "first 1.0.0 (path+file:[..]first[..]Scarb.toml)".to_string(),
        "foo 2.0.0 (git+file:[..])".to_string(),
        "second 1.0.0 (path+file:[..]second[..]Scarb.toml)".to_string(),
        "third 1.0.0 (path+file:[..]third[..]Scarb.toml)".to_string(),
    ];
    for (expected, real) in zip(&expected, packages) {
        Assert::new().eq(real, expected);
    }
    Scarb::quick_command()
        .arg("fetch")
        .current_dir(&t)
        .assert()
        .success()
        .stdout_eq(indoc! {r#"
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

    Scarb::quick_command()
        .arg("package")
        .arg("--no-metadata")
        .arg("--no-verify")
        .current_dir(&hello)
        .assert()
        .failure()
        .stdout_eq(indoc! {r#"
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
    Scarb::quick_command()
        .arg("fetch")
        .current_dir(&t)
        .assert()
        .success()
        .stdout_eq(Data::from("").raw());
    let metadata = Scarb::quick_command()
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
        "core [..] (std)".to_string(),
        "foo 0.1.0 (path+[..]Scarb.toml)".to_string(),
    ];
    for (expected, real) in zip(&expected, packages) {
        Assert::new().eq(real, expected);
    }
}

#[test]
fn patch_builtin_with_default_registry_keeps_std_source() {
    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("first")
        .dep("starknet", Dep.version("2.11.0"))
        .manifest_extra(formatdoc! {r#"
            [patch.scarbs-xyz]
            starknet = "{CAIRO_VERSION}"
        "#})
        .build(&t);

    // A same-registry patch overrides the builtin package version requirement, but the package
    // itself is still provided by the `std` source bundled with Scarb.
    let metadata = Scarb::quick_command()
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
        format!("core {CAIRO_VERSION} (std)"),
        "first 1.0.0 (path+file:[..]Scarb.toml)".to_string(),
        format!("starknet {CAIRO_VERSION} (std)"),
    ];
    for (expected, real) in zip(&expected, packages) {
        Assert::new().eq(real, expected);
    }
}

#[test]
fn patch_core_with_registry() {
    let mut registry = LocalRegistry::create();
    registry.publish(|t| {
        ProjectBuilder::start()
            .name("core")
            .version("2.0.0")
            .no_core()
            .build(t);
    });
    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("first")
        .build(&t.child("first"));
    WorkspaceBuilder::start()
        .add_member("first")
        .manifest_extra(formatdoc! {r#"
            [patch.scarbs-xyz]
            core = {}
        "#, Dep.version("2").registry(&registry).build()})
        .build(&t);
    // Patching `core` should redirect it away from the bundled `std` source: no
    // "unused patch" warning should be emitted, and the resolved `core` package
    // should come from the patch registry, not `std`.
    Scarb::quick_command()
        .arg("fetch")
        .current_dir(&t)
        .assert()
        .success()
        .stdout_eq(Data::from("").raw());
    let metadata = Scarb::quick_command()
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
        "core 2.0.0 (registry+file:[..])".to_string(),
        "first 1.0.0 (path+file:[..]first[..]Scarb.toml)".to_string(),
    ];
    for (expected, real) in zip(&expected, packages) {
        Assert::new().eq(real, expected);
    }
}

#[test]
fn patch_core_with_self() {
    // A regular registry package implicitly depends on `core` (the bundled `std` corelib).
    let mut registry = LocalRegistry::create();
    registry.publish(|t| {
        ProjectBuilder::start()
            .name("dep")
            .version("1.0.0")
            .build(t);
    });
    // The compiled package is itself named `core` and patches `core` to point at itself, so the
    // implicit `core` dependency of `dep` is redirected to this local package instead of the
    // bundled `std` source.
    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("core")
        .version("2.0.0")
        .no_core()
        .dep("dep", Dep.version("1").registry(&registry))
        .manifest_extra(indoc! {r#"
            [patch.scarbs-xyz]
            core = { path = "." }
        "#})
        .build(&t);
    // The first resolution writes a lockfile in which the patched `core` is recorded as a path
    // source (i.e. without a `source` entry). The second resolution reads it back, exercising the
    // locked-dependency shortcut for `dep` whose (locked) `core` dependency has no source.
    Scarb::quick_command()
        .arg("fetch")
        .current_dir(&t)
        .assert()
        .success();
    let metadata = Scarb::quick_command()
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
    // `dep`'s implicit `core` dependency must resolve to the local package, so there is a single
    // `core` in the graph (the local `2.0.0` one) and no `core` coming from the `std` source.
    let expected = vec![
        "core 2.0.0 (path+file:[..]Scarb.toml)".to_string(),
        "dep 1.0.0 (registry+file:[..])".to_string(),
    ];
    assert_eq!(packages.len(), expected.len());
    for (expected, real) in zip(&expected, packages) {
        Assert::new().eq(real, expected);
    }
}

#[test]
fn cannot_define_default_registry_both_short_and_long_name() {
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
            [patch.scarbs-xyz]
            foo = {}
            [patch."https://scarbs.xyz/"]
            foo = {}
        "#, patch.build(), patch.build()})
        .build(&t);
    Scarb::quick_command()
        .arg("fetch")
        .current_dir(&t)
        .assert()
        .failure()
        .stdout_eq(indoc! {r#"
            error: failed to parse manifest at: [..]Scarb.toml

            Caused by:
                the `[patch]` section cannot specify both `scarbs-xyz` and `https://scarbs.xyz/`
        "#});
}

#[test]
fn default_registry_patched_builtin_assert_macros_available() {
    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .dev_dep("assert_macros", Dep.version("2.11.0"))
        .dep_cairo_test()
        .manifest_extra(formatdoc! {r#"
            [patch.scarbs-xyz]
            assert_macros = "{CAIRO_VERSION}"
        "#})
        .lib_cairo(indoc! {r#"
            #[test]
            fn some() {
                assert_eq!(1, 1);
            }
        "#})
        .build(&t);
    Scarb::quick_command()
        .args(["build", "--test"])
        .current_dir(&t)
        .assert()
        .success();
}

#[test]
fn default_registry_patched_builtin_assert_macros_incompatible_requirement() {
    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .dev_dep("starknet", Dep.version("2.17.0"))
        .dep_cairo_test()
        .manifest_extra(indoc! {r#"
            [patch.scarbs-xyz]
            starknet = "=1.0.0"
        "#})
        .lib_cairo(indoc! {r#"
            #[test]
            fn some() {
            }
        "#})
        .build(&t);
    Scarb::quick_command()
        .args(["build", "--test"])
        .current_dir(&t)
        .assert()
        .failure()
        .stdout_eq(indoc! {r#"
            error: cannot get dependencies of `pkg0@1.0.0`

            Caused by:
                cannot find package `starknet =1.0.0`
        "#});
}

#[test]
fn patch_builtin_to_other_registry_is_not_rewritten() {
    let mut registry = LocalRegistry::create();
    registry.publish(|t| {
        ProjectBuilder::start()
            .name("assert_macros")
            .version(CAIRO_VERSION)
            .no_core()
            .manifest_extra(indoc! {r#"
                [cairo-plugin]
                builtin = true
            "#})
            .build(t);
    });

    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .dev_dep("assert_macros", Dep.version("2.11.0"))
        .manifest_extra(formatdoc! {r#"
            [patch.scarbs-xyz]
            assert_macros = {}
        "#, Dep.version(CAIRO_VERSION).registry(&registry).build()})
        .build(&t);

    let metadata = Scarb::quick_command()
        .arg("--json")
        .arg("metadata")
        .arg("--format-version=1")
        .current_dir(&t)
        .stdout_json::<Metadata>();
    let assert_macros = metadata
        .packages
        .into_iter()
        .find(|package| package.name == "assert_macros")
        .unwrap();
    assert_eq!(
        assert_macros.source.to_string(),
        format!("registry+{}", registry.url)
    );
}
