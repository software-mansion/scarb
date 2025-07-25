use assert_fs::TempDir;
use assert_fs::fixture::{PathChild, PathCreateDir};
use indoc::indoc;

use scarb_metadata::Metadata;
use scarb_test_support::command::{CommandExt, Scarb};
use scarb_test_support::fsx;
use scarb_test_support::project_builder::{Dep, DepBuilder, ProjectBuilder};
use scarb_test_support::registry::local::LocalRegistry;
use scarb_test_support::workspace_builder::WorkspaceBuilder;

#[test]
fn warn_on_member_without_manifest() {
    let t = TempDir::new().unwrap().child("test_workspace");
    let pkg1 = t.child("first");
    ProjectBuilder::start().name("first").build(&pkg1);
    t.child("second").create_dir_all().unwrap();
    WorkspaceBuilder::start()
        .add_member("first")
        .add_member("second")
        .build(&t);

    Scarb::quick_snapbox()
        .arg("fetch")
        .current_dir(&t)
        .assert()
        .success()
        .stdout_matches(
            "warn: workspace members definition matched path `[..]`, \
        which misses a manifest file\n",
        );
}

#[test]
fn error_on_virtual_manifest_with_dependencies() {
    let t = TempDir::new().unwrap();
    WorkspaceBuilder::start()
        .manifest_extra(indoc! {r#"
            [dependencies]
            foo = "1.0.0"
        "#})
        .build(&t);

    Scarb::quick_snapbox()
        .arg("fetch")
        .current_dir(&t)
        .assert()
        .failure()
        .stdout_matches(indoc! {r#"
            error: failed to parse manifest at: [..]

            Caused by:
                this virtual manifest specifies a [dependencies] section, which is not allowed
                help: use [workspace.dependencies] instead
        "#});
}

#[test]
fn unify_target_dir() {
    let t = TempDir::new().unwrap();
    let pkg1 = t.child("first");
    ProjectBuilder::start().name("first").build(&pkg1);
    WorkspaceBuilder::start().add_member("first").build(&t);

    // Make sure target dir is created.
    Scarb::quick_snapbox()
        .args(["build"])
        .current_dir(&pkg1)
        .assert()
        .success();

    let root_metadata = Scarb::quick_snapbox()
        .args(["--json", "metadata", "--format-version", "1"])
        .current_dir(&t)
        .stdout_json::<Metadata>();

    let pkg_metadata = Scarb::quick_snapbox()
        .args(["--json", "metadata", "--format-version", "1"])
        .current_dir(&pkg1)
        .stdout_json::<Metadata>();

    assert_eq!(root_metadata.target_dir, pkg_metadata.target_dir);
    assert_eq!(
        fsx::canonicalize(
            root_metadata
                .target_dir
                .unwrap()
                .to_owned()
                .into_std_path_buf()
        )
        .unwrap(),
        fsx::canonicalize(t.child("target")).unwrap()
    );
}

#[test]
fn target_name_duplicate() {
    let t = TempDir::new().unwrap();
    let pkg1 = t.child("first");
    ProjectBuilder::start()
        .name("first")
        .manifest_extra(indoc! {r#"
        [[target.starknet-contract]]
        name = "hello"
        "#})
        .build(&pkg1);
    let pkg2 = t.child("second");
    ProjectBuilder::start()
        .name("second")
        .manifest_extra(indoc! {r#"
        [[target.starknet-contract]]
        name = "hello"
        "#})
        .build(&pkg2);
    WorkspaceBuilder::start()
        .add_member("first")
        .add_member("second")
        .build(&t);

    Scarb::quick_snapbox()
        .arg("fetch")
        .current_dir(&t)
        .assert()
        .failure()
        .stdout_matches(indoc! {r#"
            error: workspace contains duplicate target definitions `starknet-contract (hello)`
            help: use different target names to resolve the conflict
        "#});
}

#[test]
fn inherited_deps_cannot_override_source_version() {
    let mut registry = LocalRegistry::create();
    registry.publish(|t| {
        ProjectBuilder::start()
            .name("some")
            .version("1.0.0")
            .build(t);
    });

    let t = TempDir::new().unwrap();

    let first = t.child("first");
    ProjectBuilder::start()
        .name("first")
        .dep("some", Dep.workspace().version("2.0.0"))
        .build(&first);

    WorkspaceBuilder::start()
        .add_member("first")
        .dep("some", Dep.version("1.0.0").registry(&registry))
        .build(&t);

    Scarb::quick_snapbox()
        .arg("fetch")
        .current_dir(&t)
        .assert()
        .failure()
        .stdout_matches(indoc! {r#"
        error: failed to parse manifest at: [..]Scarb.toml

        Caused by:
            TOML parse error at line 7, column 8
              |
            7 | some = { workspace = true, version = "2.0.0" }
              |        ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
            field `version` is not allowed when inheriting workspace dependency
        "#});
}

#[test]
fn inherited_deps_cannot_override_default_features_flag() {
    let mut registry = LocalRegistry::create();
    registry.publish(|t| {
        ProjectBuilder::start()
            .name("some")
            .version("1.0.0")
            .build(t);
    });

    let t = TempDir::new().unwrap();

    let first = t.child("first");
    ProjectBuilder::start()
        .name("first")
        .dep("some", Dep.workspace().default_features(false))
        .build(&first);

    WorkspaceBuilder::start()
        .add_member("first")
        .dep("some", Dep.version("1.0.0").registry(&registry))
        .build(&t);

    Scarb::quick_snapbox()
        .arg("fetch")
        .current_dir(&t)
        .assert()
        .failure()
        .stdout_matches(indoc! {r#"
        error: failed to parse manifest at: [..]Scarb.toml

        Caused by:
            TOML parse error at line 7, column 8
              |
            7 | some = { workspace = true, default-features = false }
              |        ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
            field `default-features` is not allowed when inheriting workspace dependency
        "#});
}

#[test]
fn inherited_deps_can_override_features_list() {
    let mut registry = LocalRegistry::create();
    registry.publish(|t| {
        ProjectBuilder::start()
            .name("some")
            .version("1.0.0")
            .manifest_extra(indoc! {r#"
                [features]
                some = []
                other = []
            "#})
            .build(t);
    });

    let t = TempDir::new().unwrap();

    let first = t.child("first");
    ProjectBuilder::start()
        .name("first")
        .dep("some", Dep.workspace().features(vec!["some"].into_iter()))
        .build(&first);

    WorkspaceBuilder::start()
        .add_member("first")
        .dep(
            "some",
            Dep.version("1.0.0")
                .registry(&registry)
                .features(vec!["other"].into_iter()),
        )
        .build(&t);

    let meta = Scarb::quick_snapbox()
        .arg("--json")
        .arg("metadata")
        .arg("--format-version=1")
        .current_dir(&t)
        .stdout_json::<Metadata>();

    let first = meta
        .packages
        .into_iter()
        .find(|p| p.name == "first")
        .unwrap();

    let dep = first
        .dependencies
        .into_iter()
        .find(|p| p.name == "some")
        .unwrap();
    // Note, that the workspace declares `features = ["other"]`,
    // but the member extends it with `features = ["some"]`.
    assert_eq!(
        dep.features,
        Some(vec!["other".to_string(), "some".to_string()])
    );
}

#[test]
fn warn_on_compiler_config_in_ws_member() {
    let t = TempDir::new().unwrap().child("test_workspace");
    ProjectBuilder::start()
        .name("first")
        .manifest_extra(indoc! {r#"
            [cairo]
            sierra-replace-ids = true
        "#})
        .build(&t.child("first"));
    WorkspaceBuilder::start()
        .add_member("first")
        .add_member("second")
        .build(&t);
    Scarb::quick_snapbox()
        .arg("fetch")
        .current_dir(&t)
        .assert()
        .success()
        .stdout_eq(indoc!{r#"
            warn: in context of a workspace, only the `profile` set in the workspace manifest is applied,
            but the `first` package also defines `profile` in the manifest

        "#});
}
