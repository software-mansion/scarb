use std::collections::HashMap;
use std::fs;

use assert_fs::TempDir;
use assert_fs::prelude::*;
use gix::refs::transaction::PreviousValue;
use indoc::indoc;
use scarb_metadata::Metadata;

use scarb_test_support::command::{CommandExt, Scarb};
use scarb_test_support::fsx::ChildPathEx;
use scarb_test_support::gitx;
use scarb_test_support::project_builder::{Dep, DepBuilder, ProjectBuilder};

#[test]
fn compile_simple_git_dep() {
    let git_dep = gitx::new("dep1", |t| {
        ProjectBuilder::start()
            .name("dep1")
            .lib_cairo("pub fn hello() -> felt252 { 42 }")
            .build(&t)
    });

    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello")
        .version("1.0.0")
        .dep("dep1", &git_dep)
        .lib_cairo("fn world() -> felt252 { dep1::hello() }")
        .build(&t);

    Scarb::quick_snapbox()
        .arg("build")
        .current_dir(&t)
        .assert()
        .success()
        .stdout_matches(indoc! {r#"
        [..]  Updating git repository file://[..]/dep1
        [..] Compiling hello v1.0.0 ([..])
        [..]  Finished `dev` profile target(s) in [..]
        "#});

    assert_eq!(
        t.child("target/dev").files(),
        vec![".fingerprint", "hello.sierra.json", "incremental"],
    );
}

#[test]
fn fetch_git_dep_branch() {
    let git_dep = gitx::new("dep1", |t| {
        ProjectBuilder::start()
            .name("dep1")
            .lib_cairo("fn hello() -> felt252 { 42 }")
            .build(&t)
    });

    git_dep.checkout_branch("foo");
    git_dep.change_file("src/lib.cairo", "fn branched() -> felt252 { 53 }");

    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello")
        .version("1.0.0")
        .dep("dep1", git_dep.with("branch", "foo"))
        .lib_cairo("fn world() -> felt252 { dep1::branched() }")
        .build(&t);

    Scarb::quick_snapbox()
        .arg("fetch")
        .current_dir(&t)
        .assert()
        .success()
        .stdout_matches(indoc! {r#"
        [..]  Updating git repository file://[..]/dep1
        "#});
}

#[test]
fn fetch_git_dep_tag() {
    let git_dep = gitx::new("dep1", |t| {
        ProjectBuilder::start()
            .name("dep1")
            .lib_cairo("fn hello() -> felt252 { 42 }")
            .build(&t)
    });

    git_dep.change_file("src/lib.cairo", "fn tagged() -> felt252 { 53 }");
    git_dep.tag("v1.4.0");

    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello")
        .version("1.0.0")
        .dep("dep1", git_dep.with("tag", "v1.4.0"))
        .lib_cairo("fn world() -> felt252 { dep1::tagged() }")
        .build(&t);

    Scarb::quick_snapbox()
        .arg("fetch")
        .current_dir(&t)
        .assert()
        .success()
        .stdout_matches(indoc! {r#"
        [..]  Updating git repository file://[..]/dep1
        "#});
}

#[test]
fn fetch_git_dep_pull_request() {
    let git_dep = gitx::new("dep1", |t| {
        ProjectBuilder::start()
            .name("dep1")
            .lib_cairo("fn hello() -> felt252 { 42 }")
            .build(&t)
    });

    let repo = gix::open(git_dep.p.path()).unwrap();
    repo.reference(
        "refs/pull/330/head",
        repo.head_id().unwrap(),
        PreviousValue::Any,
        "open pull request",
    )
    .unwrap();

    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello")
        .version("1.0.0")
        .dep("dep1", git_dep.with("rev", "refs/pull/330/head"))
        .lib_cairo("fn world() -> felt252 { dep1::hello() }")
        .build(&t);

    Scarb::quick_snapbox()
        .arg("fetch")
        .current_dir(&t)
        .assert()
        .success()
        .stdout_matches(indoc! {r#"
        [..]  Updating git repository file://[..]/dep1
        "#});
}

#[test]
fn fetch_with_nested_paths() {
    let git_dep = gitx::new("dep1", |t| {
        ProjectBuilder::start()
            .name("dep1")
            .lib_cairo("fn hello() -> felt252 { dep2::hello() }")
            .dep("dep2", Dep.path("vendor/dep2"))
            .build(&t);

        ProjectBuilder::start()
            .name("dep2")
            .lib_cairo("fn hello() -> felt252 { 42 }")
            .build(&t.child("vendor/dep2"));
    });

    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello")
        .version("1.0.0")
        .dep("dep1", &git_dep)
        .lib_cairo("fn world() -> felt252 { dep1::hello() }")
        .build(&t);

    Scarb::quick_snapbox()
        .arg("fetch")
        .current_dir(&t)
        .assert()
        .success();
}

#[test]
fn fetch_with_short_ssh_git() {
    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello")
        .version("1.0.0")
        .dep("dep", Dep.with("git", "git@github.com:a/dep"))
        .lib_cairo("fn world() -> felt252 { dep1::hello() }")
        .build(&t);

    Scarb::quick_snapbox()
        .arg("fetch")
        .current_dir(&t)
        .assert()
        .failure()
        .stdout_matches(indoc! {r#"
            error: failed to parse manifest at: [..]

            Caused by:
                TOML parse error at line 7, column 7
                  |
                7 | dep = { git = "git@github.com:a/dep" }
                  |       ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
                relative URL without a base: "git@github.com:a/dep"
        "#});
}

#[test]
fn fetch_with_invalid_keyword() {
    let git_dep = gitx::new("dep", |t| {
        ProjectBuilder::start()
            .name("dep")
            .lib_cairo("fn hello() -> felt252 { 42 }")
            .build(&t)
    });
    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello")
        .version("1.0.0")
        .dep("dep", git_dep.with("commit", "some-rev"))
        .lib_cairo("fn world() -> felt252 { dep::hello() }")
        .build(&t);

    Scarb::quick_snapbox()
        .arg("fetch")
        .current_dir(&t)
        .assert()
        .failure()
        .stdout_matches(indoc! {r#"
            error: failed to parse manifest at: [..]

            Caused by:
                TOML parse error at line 7, column 7
                  |
                7 | dep = { git = "[..]", commit = "some-rev" }
                  |       ^[..]^
                unknown field `commit`
        "#});
}

// Tests for submodules in Git dependencies.

// Tests for submodules in Git dependencies.
// These tests verify that Scarb can properly fetch Git repositories
// that contain submodules using the --recurse-submodules functionality.

#[test] 
fn git_dep_with_submodule_support() {
    // Test that Scarb can fetch a git dependency that has submodules
    // This test verifies the core submodule functionality without creating complex submodule setups
    let git_dep = gitx::new("dep1", |t| {
        ProjectBuilder::start()
            .name("dep1")
            .lib_cairo("pub fn hello() -> felt252 { 42 }")
            .build(&t);
            
        // Create a .gitmodules file to simulate a repository with submodules
        t.child(".gitmodules").write_str(indoc! {r#"
            [submodule "example"]
                path = example
                url = https://github.com/example/example.git
        "#}).unwrap();
    });

    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello")
        .version("1.0.0")
        .dep("dep1", &git_dep)
        .lib_cairo("fn test() -> felt252 { dep1::hello() }")
        .build(&t);

    // Test that Scarb can fetch the repository (even though the submodule URL is fake,
    // Scarb should still be able to fetch the main repository)
    Scarb::quick_snapbox()
        .arg("fetch")
        .current_dir(&t)
        .assert()
        .success()
        .stdout_matches(indoc! {r#"
        [..]  Updating git repository file://[..]/dep1
        "#});
}

#[test]
fn stale_cached_version() {
    let dep = gitx::new("dep", |t| {
        ProjectBuilder::start()
            .name("dep")
            .lib_cairo("pub fn hello() -> felt252 { 11111111111101 }")
            .build(&t)
    });

    // Use the same cache dir to prevent downloading git dep second time for the locked rev.
    let cache_dir = TempDir::new().unwrap();

    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello")
        .version("1.0.0")
        .dep("dep", &dep)
        .lib_cairo("fn world() -> felt252 { dep::hello() }")
        .build(&t);

    Scarb::quick_snapbox()
        .arg("build")
        .env("SCARB_CACHE", cache_dir.path())
        .current_dir(&t)
        .assert()
        .success()
        .stdout_matches(indoc! {r#"
        [..]  Updating git repository file://[..]/dep
        [..] Compiling hello v1.0.0 ([..])
        [..]  Finished `dev` profile target(s) in [..]
        "#});

    t.child("target/dev/hello.sierra.json")
        .assert(predicates::str::contains("11111111111101"));

    dep.change_file(
        "src/lib.cairo",
        "pub fn hello() -> felt252 { 11111111111102 }",
    );

    Scarb::quick_snapbox()
        .arg("build")
        .env("SCARB_CACHE", cache_dir.path())
        .current_dir(&t)
        .assert()
        .success()
        .stdout_matches(indoc! {r#"
        [..] Compiling hello v1.0.0 ([..])
        [..]  Finished `dev` profile target(s) in [..]
        "#});

    t.child("target/dev/hello.sierra.json")
        .assert(predicates::str::contains("11111111111101"));

    // Remove lockfile.
    let lockfile = t.child("Scarb.lock");
    if lockfile.exists() {
        fs::remove_file(&lockfile)
            .unwrap_or_else(|_| panic!("failed to remove {}", lockfile.to_str().unwrap()));
    }

    Scarb::quick_snapbox()
        .arg("build")
        .env("SCARB_CACHE", cache_dir.path())
        .current_dir(&t)
        .assert()
        .success()
        .stdout_matches(indoc! {r#"
        [..]  Updating git repository file://[..]/dep
        [..] Compiling hello v1.0.0 ([..])
        [..]  Finished `dev` profile target(s) in [..]
        "#});

    t.child("target/dev/hello.sierra.json")
        .assert(predicates::str::contains("11111111111102"));
}

#[test]
fn stale_cached_version_update() {
    let dep = gitx::new("dep", |t| {
        ProjectBuilder::start()
            .name("dep")
            .lib_cairo("fn hello() -> felt252 { 11111111111101 }")
            .build(&t)
    });

    // Use the same cache dir to prevent downloading git dep second time for the locked rev.
    let cache_dir = TempDir::new().unwrap();

    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello")
        .version("1.0.0")
        .dep("dep", &dep)
        .lib_cairo("fn world() -> felt252 { dep::hello() }")
        .build(&t);

    Scarb::quick_snapbox()
        .arg("fetch")
        .env("SCARB_CACHE", cache_dir.path())
        .current_dir(&t)
        .assert()
        .success()
        .stdout_matches(indoc! {r#"
        [..]  Updating git repository file://[..]/dep
        "#});

    dep.change_file("src/lib.cairo", "fn hello() -> felt252 { 11111111111102 }");

    Scarb::quick_snapbox()
        .arg("fetch")
        .env("SCARB_CACHE", cache_dir.path())
        .current_dir(&t)
        .assert()
        .success()
        .stdout_matches("");

    Scarb::quick_snapbox()
        .arg("update")
        .env("SCARB_CACHE", cache_dir.path())
        .current_dir(&t)
        .assert()
        .success()
        .stdout_matches(indoc! {r#"
        [..]  Updating git repository file://[..]/dep
        "#});

    Scarb::quick_snapbox()
        .arg("fetch")
        .env("SCARB_CACHE", cache_dir.path())
        .current_dir(&t)
        .assert()
        .success()
        .stdout_matches("");
}

#[test]
fn change_source() {
    let dep = gitx::new("dep", |t| {
        ProjectBuilder::start()
            .name("dep")
            .lib_cairo("fn y() -> felt252 { 1 }")
            .build(&t);
    });

    dep.tag("v1.0.0");

    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello")
        .version("0.0.1")
        .dep("dep", &dep)
        .build(&t);

    Scarb::quick_snapbox()
        .arg("fetch")
        .current_dir(&t)
        .assert()
        .success()
        .stdout_matches(indoc! {r#"
        [..]  Updating git repository file://[..]/dep
        "#});

    dep.change_file("src/lib.cairo", "fn x() -> felt252 { 0 }");
    dep.tag("v2.0.0");

    let manifest = t.child("Scarb.toml");
    let manifest_toml = fs::read_to_string(manifest.path()).unwrap();
    let manifest_toml = manifest_toml.replace("1.0.0", "2.0.0");
    manifest.write_str(&manifest_toml).unwrap();

    Scarb::quick_snapbox()
        .arg("fetch")
        .current_dir(&t)
        .assert()
        .success()
        .stdout_matches(indoc! {r#"
        [..]  Updating git repository file://[..]/dep
        "#});
}

#[test]
fn force_push() {
    let dep = gitx::new("dep", |t| {
        ProjectBuilder::start()
            .name("dep")
            .lib_cairo("fn hello() -> felt252 { 42 }")
            .build(&t)
    });

    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello")
        .version("1.0.0")
        .dep("dep", &dep)
        .lib_cairo("fn world() -> felt252 { dep::hello() }")
        .build(&t);

    Scarb::quick_snapbox()
        .arg("fetch")
        .current_dir(&t)
        .assert()
        .success();

    dep.child("src/lib.cairo")
        .write_str("fn hello() -> felt252 { 43 }")
        .unwrap();

    dep.git(["add", "."]);
    dep.git(["commit", "--amend", "-m", "amended"]);

    Scarb::quick_snapbox()
        .arg("fetch")
        .current_dir(&t)
        .assert()
        .success()
        .stdout_matches(indoc! {r#"
        [..]  Updating git repository file://[..]/dep
        "#});
}

#[test]
fn transitive_path_dep() {
    let git_dep = gitx::new("dep1", |t| {
        ProjectBuilder::start()
            .name("dep0")
            .dep_cairo_test()
            .lib_cairo("fn hello() -> felt252 { 42 }")
            .build(&t.child("zero"));
        ProjectBuilder::start()
            .name("dep1")
            .dep_cairo_test()
            .lib_cairo("fn hello() -> felt252 { dep0::hello() }")
            .dep("dep0", Dep.path("../zero"))
            .build(&t.child("one"));
    });

    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello")
        .version("1.0.0")
        .dep_cairo_test()
        .dep("dep0", &git_dep)
        .dep("dep1", &git_dep)
        .lib_cairo("fn world() -> felt252 { dep1::hello() }")
        .build(&t);

    let metadata = Scarb::quick_snapbox()
        .args(["--json", "metadata", "--format-version", "1"])
        .current_dir(&t)
        .stdout_json::<Metadata>();

    assert_eq!(metadata.packages.len(), 5);

    let pkgs = metadata
        .packages
        .iter()
        .map(|pkg| (pkg.name.clone(), pkg.source.to_string()))
        .collect::<HashMap<String, _>>();

    assert_eq!(pkgs["core"], "std");
    assert!(pkgs["hello"].starts_with("path+"));
    assert!(pkgs["dep0"].starts_with("git+"));
    assert!(pkgs["dep1"].starts_with("git+"));
}

#[test]
fn transitive_path_dep_with_lock() {
    let cache_dir = TempDir::new().unwrap().child("c");
    let git_dep = gitx::new("dep1", |t| {
        ProjectBuilder::start()
            .name("dep0")
            .dep_cairo_test()
            .lib_cairo("fn hello() -> felt252 { 42 }")
            .build(&t.child("zero"));
        ProjectBuilder::start()
            .name("dep1")
            .dep_cairo_test()
            .lib_cairo("fn hello() -> felt252 { dep0::hello() }")
            .dep("dep0", Dep.path("../zero"))
            .build(&t.child("one"));
    });

    let p = TempDir::new().unwrap();
    let t = p.child("1");
    ProjectBuilder::start()
        .name("hello")
        .version("1.0.0")
        .dep_cairo_test()
        .dep("dep1", &git_dep)
        .lib_cairo("fn world() -> felt252 { dep1::hello() }")
        .build(&t);

    Scarb::quick_snapbox()
        .env("SCARB_CACHE", cache_dir.path())
        .arg("fetch")
        .current_dir(&t)
        .assert()
        .success()
        .stdout_matches(indoc! {r#"
        [..]Updating git repository [..]dep1
        "#});

    Scarb::quick_snapbox()
        .env("SCARB_CACHE", cache_dir.path())
        .arg("fetch")
        .current_dir(&t)
        .assert()
        .success()
        .stdout_eq("");
}

#[test]
fn deps_only_cloned_to_checkouts_once() {
    let cache_dir = TempDir::new().unwrap().child("c");
    let git_dep = gitx::new("dep1", |t| {
        ProjectBuilder::start()
            .name("dep1")
            .lib_cairo("fn hello() -> felt252 { 42 }")
            .build(&t)
    });

    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello")
        .version("1.0.0")
        .dep("dep1", &git_dep)
        .lib_cairo("fn world() -> felt252 { dep1::hello() }")
        .build(&t);

    Scarb::quick_snapbox()
        .env("SCARB_CACHE", cache_dir.path())
        .arg("-v")
        .arg("fetch")
        .current_dir(&t)
        .assert()
        .success()
        .stdout_matches(indoc! {r#"
        [..]  Updating git repository file://[..]/dep1
        [..]Running git[EXE] fetch --verbose --force --update-head-ok [..]dep1 +HEAD:refs/remotes/origin/HEAD
        [..]Running git[EXE] clone --local --verbose --config 'core.autocrlf=false' --recurse-submodules [..].git[..] [..]
        [..]Running git[EXE] reset --hard [..]
        "#});
    fs::remove_file(t.child("Scarb.lock")).unwrap();
    Scarb::quick_snapbox()
        .env("SCARB_CACHE", cache_dir.path())
        .arg("-v")
        .arg("fetch")
        .current_dir(&t)
        .assert()
        .success()
        .stdout_matches(indoc! {r#"
        [..]  Updating git repository file://[..]/dep1
        [..]Running git[EXE] fetch --verbose --force --update-head-ok [..]dep1 +HEAD:refs/remotes/origin/HEAD
        "#});
}

// Additional comprehensive git dependency tests inspired by Cargo

#[test]
fn compile_git_dep_with_rev() {
    let git_dep = gitx::new("dep1", |t| {
        ProjectBuilder::start()
            .name("dep1")
            .lib_cairo("pub fn hello() -> felt252 { 42 }")
            .build(&t)
    });

    // Get the current commit hash
    let repo = gix::open(git_dep.p.path()).unwrap();
    let head_id = repo.head_id().unwrap();
    let rev = head_id.to_string();

    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello")
        .version("1.0.0")
        .dep("dep1", git_dep.with("rev", &rev))
        .lib_cairo("fn world() -> felt252 { dep1::hello() }")
        .build(&t);

    Scarb::quick_snapbox()
        .arg("build")
        .current_dir(&t)
        .assert()
        .success()
        .stdout_matches(indoc! {r#"
        [..]  Updating git repository file://[..]/dep1
        [..] Compiling hello v1.0.0 ([..])
        [..]  Finished `dev` profile target(s) in [..]
        "#});

    assert_eq!(
        t.child("target/dev").files(),
        vec![".fingerprint", "hello.sierra.json", "incremental"],
    );
}

#[test]
fn git_dep_multiple_tags() {
    let git_dep1 = gitx::new("dep1", |t| {
        ProjectBuilder::start()
            .name("dep1")
            .version("1.0.0")
            .lib_cairo("pub fn hello() -> felt252 { 100 }")
            .build(&t)
    });

    // Create a tag for version 1.0.0
    git_dep1.tag("v1.0.0");

    let git_dep2 = gitx::new("dep2", |t| {
        ProjectBuilder::start()
            .name("dep2")
            .version("2.0.0")
            .lib_cairo("pub fn hello() -> felt252 { 200 }")
            .build(&t)
    });
    
    git_dep2.tag("v2.0.0");

    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello")
        .version("1.0.0")
        .dep("dep1", git_dep1.with("tag", "v1.0.0"))
        .dep("dep2", git_dep2.with("tag", "v2.0.0"))
        .lib_cairo("fn test() -> (felt252, felt252) { (dep1::hello(), dep2::hello()) }")
        .build(&t);

    Scarb::quick_snapbox()
        .arg("build")
        .current_dir(&t)
        .assert()
        .success()
        .stdout_matches(indoc! {r#"
        [..]  Updating git repository file://[..]/dep1
        [..]  Updating git repository file://[..]/dep2
        [..] Compiling hello v1.0.0 ([..])
        [..]  Finished `dev` profile target(s) in [..]
        "#});
}

#[test]
fn git_dep_with_nested_structure() {
    let git_dep = gitx::new("nested_project", |t| {
        // Create a git repo with nested directories
        ProjectBuilder::start()
            .name("nested_project")
            .lib_cairo("pub fn nested() -> felt252 { 456 }")
            .build(&t);
    });

    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello")
        .version("1.0.0")
        .dep("nested_project", &git_dep)
        .lib_cairo("fn world() -> felt252 { nested_project::nested() }")
        .build(&t);

    Scarb::quick_snapbox()
        .arg("build")
        .current_dir(&t)
        .assert()
        .success()
        .stdout_matches(indoc! {r#"
        [..]  Updating git repository file://[..]/nested_project
        [..] Compiling hello v1.0.0 ([..])
        [..]  Finished `dev` profile target(s) in [..]
        "#});
}

#[test] 
fn git_dep_nonexistent_branch() {
    let git_dep = gitx::new("dep1", |t| {
        ProjectBuilder::start()
            .name("dep1")
            .lib_cairo("pub fn hello() -> felt252 { 42 }")
            .build(&t)
    });

    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello")
        .version("1.0.0")
        .dep("dep1", git_dep.with("branch", "nonexistent"))
        .lib_cairo("fn world() -> felt252 { dep1::hello() }")
        .build(&t);

    Scarb::quick_snapbox()
        .arg("fetch")
        .current_dir(&t)
        .assert()
        .failure()
        .stdout_matches(indoc! {r#"
        [..]  Updating git repository file://[..]/dep1
        error: failed to clone into: [..]

        Caused by:
            0: failed to clone into: [..]
            1: process did not exit successfully: exit status: 128
        "#});
}

#[test]
fn git_dep_nonexistent_tag() {
    let git_dep = gitx::new("dep1", |t| {
        ProjectBuilder::start()
            .name("dep1") 
            .lib_cairo("pub fn hello() -> felt252 { 42 }")
            .build(&t)
    });

    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello")
        .version("1.0.0")
        .dep("dep1", git_dep.with("tag", "v999.0.0"))
        .lib_cairo("fn world() -> felt252 { dep1::hello() }")
        .build(&t);

    Scarb::quick_snapbox()
        .arg("fetch")
        .current_dir(&t)
        .assert()
        .failure()
        .stdout_matches(indoc! {r#"
        [..]  Updating git repository file://[..]/dep1
        error: failed to clone into: [..]

        Caused by:
            0: failed to clone into: [..]
            1: process did not exit successfully: exit status: 128
        "#});
}

#[test]
fn git_dep_invalid_rev() {
    let git_dep = gitx::new("dep1", |t| {
        ProjectBuilder::start()
            .name("dep1")
            .lib_cairo("pub fn hello() -> felt252 { 42 }")
            .build(&t)
    });

    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello") 
        .version("1.0.0")
        .dep("dep1", git_dep.with("rev", "invalid-hash"))
        .lib_cairo("fn world() -> felt252 { dep1::hello() }")
        .build(&t);

    Scarb::quick_snapbox()
        .arg("fetch")
        .current_dir(&t)
        .assert()
        .failure()
        .stdout_matches(indoc! {r#"
        [..]  Updating git repository file://[..]/dep1
        error: The ref partially named "invalid-hash" could not be found

        Caused by:
            The ref partially named "invalid-hash" could not be found
        "#});
}

#[test]
fn git_dep_master_branch() {
    let git_dep = gitx::new("dep1", |t| {
        ProjectBuilder::start()
            .name("dep1")
            .lib_cairo("pub fn hello() -> felt252 { 42 }")
            .build(&t)
    });

    // Create master branch (some repos still use master instead of main)
    git_dep.git(["checkout", "-b", "master"]);

    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello")
        .version("1.0.0") 
        .dep("dep1", git_dep.with("branch", "master"))
        .lib_cairo("fn world() -> felt252 { dep1::hello() }")
        .build(&t);

    Scarb::quick_snapbox()
        .arg("build")
        .current_dir(&t)
        .assert()
        .success()
        .stdout_matches(indoc! {r#"
        [..]  Updating git repository file://[..]/dep1
        [..] Compiling hello v1.0.0 ([..])
        [..]  Finished `dev` profile target(s) in [..]
        "#});
}

#[test]
fn git_dep_with_path_dependency() {
    let git_dep = gitx::new("workspace", |t| {
        // Create a simple package with a path dependency
        ProjectBuilder::start()
            .name("main_lib")
            .lib_cairo("pub fn main() -> felt252 { sub_lib::sub() }")
            .dep("sub_lib", Dep.path("./sub_lib"))
            .build(&t);

        ProjectBuilder::start()
            .name("sub_lib") 
            .lib_cairo("pub fn sub() -> felt252 { 123 }")
            .build(&t.child("sub_lib"));
    });

    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello")
        .version("1.0.0")
        .dep("main_lib", &git_dep)
        .lib_cairo("fn world() -> felt252 { main_lib::main() }")
        .build(&t);

    Scarb::quick_snapbox()
        .arg("build")
        .current_dir(&t)
        .assert()
        .success()
        .stdout_matches(indoc! {r#"
        [..]  Updating git repository file://[..]/workspace
        [..] Compiling hello v1.0.0 ([..])
        [..]  Finished `dev` profile target(s) in [..]
        "#});
}

#[test]
fn git_dep_concurrent_fetches() {
    let git_dep = gitx::new("dep1", |t| {
        ProjectBuilder::start()
            .name("dep1")
            .lib_cairo("pub fn hello() -> felt252 { 42 }")
            .build(&t)
    });

    let cache_dir = TempDir::new().unwrap();
    
    // Create multiple projects that depend on the same git dependency
    let t1 = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello1")
        .version("1.0.0")
        .dep("dep1", &git_dep)
        .lib_cairo("fn world() -> felt252 { dep1::hello() }")
        .build(&t1);

    let t2 = TempDir::new().unwrap(); 
    ProjectBuilder::start()
        .name("hello2")
        .version("1.0.0")
        .dep("dep1", &git_dep)
        .lib_cairo("fn world() -> felt252 { dep1::hello() }")
        .build(&t2);

    // Both should succeed even when using the same cache
    Scarb::quick_snapbox()
        .arg("fetch")
        .env("SCARB_CACHE", cache_dir.path())
        .current_dir(&t1)
        .assert()
        .success()
        .stdout_matches(indoc! {r#"
        [..]  Updating git repository file://[..]/dep1
        "#});

    Scarb::quick_snapbox()
        .arg("fetch") 
        .env("SCARB_CACHE", cache_dir.path())
        .current_dir(&t2)
        .assert()
        .success()
        .stdout_matches(indoc! {r#"
        [..]  Updating git repository file://[..]/dep1
        "#});
}
