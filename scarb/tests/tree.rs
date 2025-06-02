use assert_fs::TempDir;
use assert_fs::prelude::*;
use indoc::indoc;

use scarb_test_support::command::Scarb;
use scarb_test_support::project_builder::{Dep, DepBuilder, ProjectBuilder};
use scarb_test_support::workspace_builder::WorkspaceBuilder;

#[test]
fn simple() {
    let t = TempDir::new().unwrap();

    let dep1 = t.child("dep1");
    ProjectBuilder::start()
        .name("dep1")
        .version("0.1.0")
        .build(&dep1);

    let dep2 = t.child("dep2");
    ProjectBuilder::start()
        .name("dep2")
        .version("0.1.0")
        .build(&dep2);

    let root = t.child("root");
    ProjectBuilder::start()
        .name("root")
        .version("0.1.0")
        .dep("dep1", &dep1)
        .build(&root);

    Scarb::quick_snapbox()
        .arg("tree")
        .current_dir(&root)
        .assert()
        .success()
        .stdout_matches(indoc! {r#"
            root v0.1.0 ([..])
            └── dep1 v0.1.0 ([..])
        "#});
}

#[test]
fn json_output() {
    let t = TempDir::new().unwrap();

    let dep1 = t.child("dep1");
    ProjectBuilder::start()
        .name("dep1")
        .version("0.1.0")
        .build(&dep1);

    let dep2 = t.child("dep2");
    ProjectBuilder::start()
        .name("dep2")
        .version("0.1.0")
        .build(&dep2);

    let root = t.child("root");
    ProjectBuilder::start()
        .name("root")
        .version("0.1.0")
        .dep("dep1", &dep1)
        .build(&root);

    Scarb::quick_snapbox()
        .arg("--json")
        .arg("tree")
        .current_dir(&root)
        .assert()
        .success()
        .stdout_matches(indoc! {r#"
        [{"package":"root 0.1.0 ([..])","branches":[{"package":"dep1 0.1.0 ([..])"}]}]
        "#});
}

#[test]
fn requires_workspace() {
    let t = TempDir::new().unwrap();
    Scarb::quick_snapbox()
        .env("RUST_BACKTRACE", "0")
        .arg("tree")
        .current_dir(&t)
        .assert()
        .failure()
        .stdout_matches(indoc! {r#"
            error: failed to read manifest at: [..]/Scarb.toml

            Caused by:
                [..]
        "#});
}

#[test]
fn no_dedupe() {
    let t = TempDir::new().unwrap();

    let common_dep = t.child("common_dep");
    ProjectBuilder::start()
        .name("common_dep")
        .version("0.1.0")
        .build(&common_dep);

    let common = t.child("common");
    ProjectBuilder::start()
        .name("common")
        .version("0.1.0")
        .dep("common_dep", &common_dep)
        .dep_builtin("starknet")
        .build(&common);

    let dep1 = t.child("dep1");
    ProjectBuilder::start()
        .name("dep1")
        .version("0.1.0")
        .dep("common", &common)
        .build(&dep1);

    let dep2 = t.child("dep2");
    ProjectBuilder::start()
        .name("dep2")
        .version("0.1.0")
        .dep("common", &common)
        .build(&dep2);

    let root = t.child("root");
    ProjectBuilder::start()
        .name("root")
        .version("0.1.0")
        .dep("dep1", &dep1)
        .dep("dep2", &dep2)
        .build(&root);

    Scarb::quick_snapbox()
        .arg("tree")
        .arg("--no-dedupe")
        .current_dir(&root)
        .assert()
        .success()
        .stdout_matches(indoc! {r#"
            root v0.1.0 ([..])
            ├── dep1 v0.1.0 ([..])
            │   └── common v0.1.0 ([..])
            │       ├── common_dep v0.1.0 ([..])
            │       └── starknet v[..] (std)
            └── dep2 v0.1.0 ([..])
                └── common v0.1.0 ([..])
                    ├── common_dep v0.1.0 ([..])
                    └── starknet v[..] (std)
        "#});
}

#[test]
fn depth() {
    let t = TempDir::new().unwrap();

    // Create a deep dependency tree: dep0 -> dep1 -> dep2 -> dep3 -> dep4.
    for i in (0..=4).rev() {
        let dep = t.child(format!("dep{}", i));
        let mut b = ProjectBuilder::start()
            .name(format!("dep{}", i))
            .version("0.1.0");

        if i < 4 {
            b = b.dep(
                format!("dep{}", i + 1),
                Dep.path(format!("../dep{}", i + 1)),
            );
        }

        b.build(&dep);
    }

    Scarb::quick_snapbox()
        .arg("tree")
        .arg("--depth=2")
        .current_dir(t.child("dep0"))
        .assert()
        .success()
        .stdout_matches(indoc! {r#"
            dep0 v0.1.0 ([..])
            └── dep1 v0.1.0 ([..])
                └── dep2 v0.1.0 ([..])
                    └── ...
        "#});

    Scarb::quick_snapbox()
        .arg("tree")
        .arg("--depth=1")
        .current_dir(t.child("dep0"))
        .assert()
        .success()
        .stdout_matches(indoc! {r#"
            dep0 v0.1.0 ([..])
            └── dep1 v0.1.0 ([..])
                └── ...
        "#});

    Scarb::quick_snapbox()
        .arg("tree")
        .arg("--depth=0")
        .current_dir(t.child("dep0"))
        .assert()
        .success()
        .stdout_matches(indoc! {r#"
            dep0 v0.1.0 ([..])
            └── ...
        "#});
}

#[test]
fn prune() {
    let t = TempDir::new().unwrap();

    // Create a dependency tree with multiple branches:
    // root -> dep1 -> dep3
    //      -> dep2 -> dep3
    let dep3 = t.child("dep3");
    ProjectBuilder::start()
        .name("dep3")
        .version("0.1.0")
        .build(&dep3);

    let dep1 = t.child("dep1");
    ProjectBuilder::start()
        .name("dep1")
        .version("0.1.0")
        .dep("dep3", &dep3)
        .build(&dep1);

    let dep2 = t.child("dep2");
    ProjectBuilder::start()
        .name("dep2")
        .version("0.1.0")
        .dep("dep3", &dep3)
        .build(&dep2);

    let root = t.child("root");
    ProjectBuilder::start()
        .name("root")
        .version("0.1.0")
        .dep("dep1", &dep1)
        .dep("dep2", &dep2)
        .build(&root);

    // Test with --prune=dep1 (should exclude dep1 and its dependencies).
    Scarb::quick_snapbox()
        .arg("tree")
        .arg("--prune=dep1")
        .current_dir(&root)
        .assert()
        .success()
        .stdout_matches(indoc! {r#"
            root v0.1.0 ([..])
            └── dep2 v0.1.0 ([..])
                └── dep3 v0.1.0 ([..])
        "#});

    // Test with --prune=dep3 (should exclude dep3 from both dep1 and dep2).
    Scarb::quick_snapbox()
        .arg("tree")
        .arg("--prune=dep3")
        .current_dir(&root)
        .assert()
        .success()
        .stdout_matches(indoc! {r#"
            root v0.1.0 ([..])
            ├── dep1 v0.1.0 ([..])
            └── dep2 v0.1.0 ([..])
        "#});

    // Test with multiple prune arguments.
    Scarb::quick_snapbox()
        .arg("tree")
        .arg("--prune=dep1")
        .arg("--prune=dep3")
        .current_dir(&root)
        .assert()
        .success()
        .stdout_matches(indoc! {r#"
            root v0.1.0 ([..])
            └── dep2 v0.1.0 ([..])
        "#});
}

#[test]
fn core() {
    let t = TempDir::new().unwrap();

    let dep1 = t.child("dep1");
    ProjectBuilder::start()
        .name("dep1")
        .version("0.1.0")
        .build(&dep1);

    let dep2 = t.child("dep2");
    ProjectBuilder::start()
        .name("dep2")
        .version("0.1.0")
        .build(&dep2);

    let root = t.child("root");
    ProjectBuilder::start()
        .name("root")
        .version("0.1.0")
        .dep("dep1", &dep1)
        .build(&root);

    Scarb::quick_snapbox()
        .arg("tree")
        .arg("--core")
        .current_dir(&root)
        .assert()
        .success()
        .stdout_matches(indoc! {r#"
            root v0.1.0 ([..])
            ├── dep1 v0.1.0 ([..])
            │   └── core v[..] (std)
            └── core v[..] (std) (*)
        "#});
}

#[test]
fn beautiful_tree_formatting() {
    let t = TempDir::new().unwrap();

    // An extra dependency to be added to dep0.
    let extra = t.child("extra");
    ProjectBuilder::start()
        .name("extra")
        .version("0.1.0")
        .build(&extra);

    let dep3 = t.child("dep3");
    ProjectBuilder::start()
        .name("dep3")
        .version("0.1.0")
        .build(&dep3);

    let dep2 = t.child("dep2");
    ProjectBuilder::start()
        .name("dep2")
        .version("0.1.0")
        .dep("dep3", dep3)
        .build(&dep2);

    let dep1 = t.child("dep1");
    ProjectBuilder::start()
        .name("dep1")
        .version("0.1.0")
        .dep("dep2", dep2)
        .build(&dep1);

    let dep0 = t.child("dep0");
    ProjectBuilder::start()
        .name("dep0")
        .version("0.1.0")
        .dep("dep1", dep1)
        .dep("extra", extra)
        .build(&dep0);

    Scarb::quick_snapbox()
        .arg("tree")
        .current_dir(dep0)
        .assert()
        .success()
        .stdout_matches(indoc! {r#"
            dep0 v0.1.0 ([..])
            ├── dep1 v0.1.0 ([..])
            │   └── dep2 v0.1.0 ([..])
            │       └── dep3 v0.1.0 ([..])
            └── extra v0.1.0 ([..])
        "#});
}

#[test]
fn dev_dependencies() {
    let t = TempDir::new().unwrap();

    let normal_dep = t.child("normal_dep");
    ProjectBuilder::start()
        .name("normal_dep")
        .version("0.1.0")
        // This one shouldn't be printed because normal_dep is not a workspace member.
        .dev_dep_builtin("cairo_test")
        .build(&normal_dep);

    let root = t.child("root");
    ProjectBuilder::start()
        .name("root")
        .version("0.1.0")
        .dep("normal_dep", &normal_dep)
        .dev_dep_builtin("assert_macros")
        .build(&root);

    Scarb::quick_snapbox()
        .arg("tree")
        .current_dir(&root)
        .assert()
        .success()
        .stdout_matches(indoc! {r#"
            root v0.1.0 ([..])
            ├── normal_dep v0.1.0 ([..])
            └── [dev-dependencies]
                └── assert_macros v[..] (std)
        "#});
}

#[test]
fn workspace_members() {
    let t = TempDir::new().unwrap();

    let common = t.child("common");
    ProjectBuilder::start()
        .name("common")
        .version("0.1.0")
        .build(&common);

    let member1 = t.child("member1");
    ProjectBuilder::start()
        .name("member1")
        .version("0.1.0")
        .dep("common", &common)
        .build(&member1);

    let member2 = t.child("member2");
    ProjectBuilder::start()
        .name("member2")
        .version("0.1.0")
        .dep("common", &common)
        .dev_dep_builtin("starknet")
        .build(&member2);

    WorkspaceBuilder::start()
        .add_member("member1")
        .add_member("member2")
        .build(&t);

    Scarb::quick_snapbox()
        .arg("tree")
        .current_dir(&t)
        .assert()
        .success()
        .stdout_matches(indoc! {r#"
            member1 v0.1.0 ([..])
            └── common v0.1.0 ([..])
            
            member2 v0.1.0 ([..])
            ├── common v0.1.0 ([..]) (*)
            └── [dev-dependencies]
                └── starknet v[..] (std)
        "#});
}

#[test]
fn cycle() {
    let t = TempDir::new().unwrap();

    let root = t.child("root");
    let dep1 = t.child("dep1");
    ProjectBuilder::start()
        .name("dep1")
        .version("0.1.0")
        .dep("root", &root)
        .build(&dep1);

    ProjectBuilder::start()
        .name("root")
        .version("0.1.0")
        .dep("dep1", &dep1)
        .build(&root);

    Scarb::quick_snapbox()
        .arg("tree")
        .current_dir(&root)
        .assert()
        .success()
        .stdout_matches(indoc! {r#"
            root v0.1.0 ([..])
            └── dep1 v0.1.0 ([..])
                └── root v0.1.0 ([..]) (*)
        "#});
}

#[test]
fn no_dedupe_cycle() {
    let t = TempDir::new().unwrap();

    let root = t.child("root");
    let dep1 = t.child("dep1");
    ProjectBuilder::start()
        .name("dep1")
        .version("0.1.0")
        .dep("root", &root)
        .build(&dep1);

    ProjectBuilder::start()
        .name("root")
        .version("0.1.0")
        .dep("dep1", &dep1)
        .build(&root);

    Scarb::quick_snapbox()
        .arg("tree")
        .arg("--no-dedupe")
        .current_dir(&root)
        .assert()
        .success()
        .stdout_matches(indoc! {r#"
            root v0.1.0 ([..])
            └── dep1 v0.1.0 ([..])
                └── root v0.1.0 ([..]) (*)
        "#});
}
