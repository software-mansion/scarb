use snapbox::Data;
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

    Scarb::quick_command()
        .arg("build")
        .current_dir(&t)
        .assert()
        .success()
        .stdout_eq(indoc! {r#"
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

    Scarb::quick_command()
        .arg("fetch")
        .current_dir(&t)
        .assert()
        .success()
        .stdout_eq(indoc! {r#"
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

    Scarb::quick_command()
        .arg("fetch")
        .current_dir(&t)
        .assert()
        .success()
        .stdout_eq(indoc! {r#"
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

    Scarb::quick_command()
        .arg("fetch")
        .current_dir(&t)
        .assert()
        .success()
        .stdout_eq(indoc! {r#"
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

    Scarb::quick_command()
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

    Scarb::quick_command()
        .arg("fetch")
        .current_dir(&t)
        .assert()
        .failure()
        .stdout_eq(indoc! {r#"
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

    Scarb::quick_command()
        .arg("fetch")
        .current_dir(&t)
        .assert()
        .failure()
        .stdout_eq(indoc! {r#"
            error: failed to parse manifest at: [..]

            Caused by:
                TOML parse error at line 7, column 7
                  |
                7 | dep = { git = "[..]", commit = "some-rev" }
                  |       ^[..]^
                unknown field `commit`
        "#});
}

#[test]
fn dep_with_submodule() {
    // Create a submodule project with Cairo code that will be the `src` directory
    let submodule = gitx::new("src_submodule", |t| {
        t.child("lib.cairo")
            .write_str("pub fn hello() -> felt252 { 42 }")
            .unwrap();
    });

    // Create the main dep project without src directory initially
    let git_dep = gitx::new("dep1", |t| {
        t.child("Scarb.toml")
            .write_str(
                r#"[package]
name = "dep1"
version = "1.0.0"
edition = "2024_07"
"#,
            )
            .unwrap();
    });

    // Add submodule as src directory
    gitx::add_submodule(&git_dep, &submodule.url(), "src");
    git_dep.commit();

    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello")
        .version("1.0.0")
        .dep("dep1", &git_dep)
        .lib_cairo("fn world() -> felt252 { dep1::hello() }")
        .build(&t);

    Scarb::quick_command()
        .arg("fetch")
        .current_dir(&t)
        .assert()
        .success()
        .stdout_eq(indoc! {r#"
        [..]  Updating git repository file://[..]/dep1
        "#});
}

#[test]
fn dep_with_submodule_update_on_rev_change() {
    // This test verifies that when switching between revisions, submodule content
    // is correctly updated to match the expected state for each revision.

    // Create the submodule with initial content (v1)
    let submodule = gitx::new("submodule", |t| {
        t.child("lib.cairo")
            .write_str("pub fn value() -> felt252 { 1 }")
            .unwrap();
    });

    // Create the main dep project
    let git_dep = gitx::new("dep1", |t| {
        t.child("Scarb.toml")
            .write_str(
                r#"[package]
name = "dep1"
version = "1.0.0"
edition = "2024_07"
"#,
            )
            .unwrap();
    });

    // Add submodule and commit (this is rev v1)
    gitx::add_submodule(&git_dep, &submodule.url(), "src");
    git_dep.commit();

    // Get the v1 revision
    let rev_v1 = git_dep.rev_parse("HEAD");

    // Update submodule to v2
    submodule.change_file("lib.cairo", "pub fn value() -> felt252 { 2 }");

    // Pull the submodule update in the main project
    git_dep.git(["submodule", "update", "--remote"]);
    git_dep.commit();

    let rev_v2 = git_dep.rev_parse("HEAD");

    // Use a shared cache directory
    let cache_dir = TempDir::new().unwrap();

    // First, fetch at v2 (latest)
    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello")
        .version("1.0.0")
        .dep("dep1", &git_dep)
        .lib_cairo("fn world() -> felt252 { dep1::value() }")
        .build(&t);

    Scarb::new()
        .cache(cache_dir.path())
        .command()
        .arg("build")
        .current_dir(&t)
        .assert()
        .success();

    // Now fetch at v1 (older revision) - this tests that submodule is updated after reset
    let t2 = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello2")
        .version("1.0.0")
        .dep("dep1", git_dep.with("rev", &rev_v1))
        .lib_cairo("fn world() -> felt252 { dep1::value() }")
        .build(&t2);

    Scarb::new()
        .cache(cache_dir.path())
        .command()
        .arg("build")
        .current_dir(&t2)
        .assert()
        .success();

    // And back to v2 to verify switching works both ways
    let t3 = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello3")
        .version("1.0.0")
        .dep("dep1", git_dep.with("rev", &rev_v2))
        .lib_cairo("fn world() -> felt252 { dep1::value() }")
        .build(&t3);

    Scarb::new()
        .cache(cache_dir.path())
        .command()
        .arg("build")
        .current_dir(&t3)
        .assert()
        .success();
}

#[test]
fn dep_with_submodule_as_path_dependency() {
    // Test that a git dependency can have a submodule that provides a path dependency.
    // This is a common pattern where a repo includes vendored dependencies as submodules.

    // Create a submodule that will be a path dependency in the base project
    let deployment = gitx::new("deployment", |t| {
        t.child("Scarb.toml")
            .write_str(
                r#"[package]
name = "deployment"
version = "1.0.0"
edition = "2024_07"
"#,
            )
            .unwrap();
        t.child("src/lib.cairo")
            .write_str("pub fn deployment_func() -> felt252 { 123 }")
            .unwrap();
    });

    // Create base project that uses deployment as a path dependency
    let base = gitx::new("base", |t| {
        t.child("Scarb.toml")
            .write_str(
                r#"[package]
name = "base"
version = "1.0.0"
edition = "2024_07"

[dependencies]
deployment = { path = "deployment" }
"#,
            )
            .unwrap();
        t.child("src/lib.cairo")
            .write_str("pub fn base_fn() -> felt252 { deployment::deployment_func() }")
            .unwrap();
    });

    // Add deployment as a submodule with absolute URL
    gitx::add_submodule(&base, &deployment.url(), "deployment");
    base.commit();

    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello")
        .version("1.0.0")
        .dep("base", &base)
        .lib_cairo("fn world() -> felt252 { base::base_fn() }")
        .build(&t);

    Scarb::quick_command()
        .arg("build")
        .current_dir(&t)
        .assert()
        .success();
}

#[test]
fn dep_with_nested_submodule() {
    // Test nested submodules: outer has a submodule that itself has a submodule.

    // Create the innermost submodule
    let inner = gitx::new("inner", |t| {
        t.child("Scarb.toml")
            .write_str(
                r#"[package]
name = "inner"
version = "1.0.0"
edition = "2024_07"
"#,
            )
            .unwrap();
        t.child("src/lib.cairo")
            .write_str("pub fn inner_fn() -> felt252 { 999 }")
            .unwrap();
    });

    // Create middle submodule that depends on inner via path
    let middle = gitx::new("middle", |t| {
        t.child("Scarb.toml")
            .write_str(
                r#"[package]
name = "middle"
version = "1.0.0"
edition = "2024_07"

[dependencies]
inner = { path = "inner" }
"#,
            )
            .unwrap();
        t.child("src/lib.cairo")
            .write_str("pub fn middle_fn() -> felt252 { inner::inner_fn() }")
            .unwrap();
    });

    // Add inner as a submodule of middle
    gitx::add_submodule(&middle, &inner.url(), "inner");
    middle.commit();

    // Create outer project that depends on middle
    let outer = gitx::new("outer", |t| {
        t.child("Scarb.toml")
            .write_str(
                r#"[package]
name = "outer"
version = "1.0.0"
edition = "2024_07"

[dependencies]
middle = { path = "middle" }
"#,
            )
            .unwrap();
        t.child("src/lib.cairo")
            .write_str("pub fn outer_fn() -> felt252 { middle::middle_fn() }")
            .unwrap();
    });

    // Add middle as a submodule of outer
    gitx::add_submodule(&outer, &middle.url(), "middle");
    outer.commit();

    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello")
        .version("1.0.0")
        .dep("outer", &outer)
        .lib_cairo("fn world() -> felt252 { outer::outer_fn() }")
        .build(&t);

    Scarb::quick_command()
        .arg("build")
        .current_dir(&t)
        .assert()
        .success();
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

    Scarb::new()
        .cache(cache_dir.path())
        .command()
        .arg("build")
        .current_dir(&t)
        .assert()
        .success()
        .stdout_eq(indoc! {r#"
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

    Scarb::new()
        .cache(cache_dir.path())
        .command()
        .arg("build")
        .current_dir(&t)
        .assert()
        .success()
        .stdout_eq(indoc! {r#"
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

    Scarb::new()
        .cache(cache_dir.path())
        .command()
        .arg("build")
        .current_dir(&t)
        .assert()
        .success()
        .stdout_eq(indoc! {r#"
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

    Scarb::new()
        .cache(cache_dir.path())
        .command()
        .arg("fetch")
        .current_dir(&t)
        .assert()
        .success()
        .stdout_eq(indoc! {r#"
        [..]  Updating git repository file://[..]/dep
        "#});

    dep.change_file("src/lib.cairo", "fn hello() -> felt252 { 11111111111102 }");

    Scarb::new()
        .cache(cache_dir.path())
        .command()
        .arg("fetch")
        .current_dir(&t)
        .assert()
        .success()
        .stdout_eq("");

    Scarb::new()
        .cache(cache_dir.path())
        .command()
        .arg("update")
        .current_dir(&t)
        .assert()
        .success()
        .stdout_eq(indoc! {r#"
        [..]  Updating git repository file://[..]/dep
        "#});

    Scarb::new()
        .cache(cache_dir.path())
        .command()
        .arg("fetch")
        .current_dir(&t)
        .assert()
        .success()
        .stdout_eq("");
}

#[test]
fn change_source() {
    let cache_dir = TempDir::new().unwrap();
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
        .dep("dep", dep.with("tag", "v1.0.0"))
        .build(&t);

    Scarb::new()
        .cache(cache_dir.path())
        .command()
        .arg("fetch")
        .current_dir(&t)
        .assert()
        .success()
        .stdout_eq(indoc! {r#"
        [..]  Updating git repository file://[..]/dep
        "#});

    dep.change_file("src/lib.cairo", "fn x() -> felt252 { 0 }");
    dep.tag("v2.0.0");

    let manifest = t.child("Scarb.toml");
    let manifest_toml = fs::read_to_string(manifest.path()).unwrap();
    let manifest_toml = manifest_toml.replace("1.0.0", "2.0.0");
    manifest.write_str(&manifest_toml).unwrap();

    Scarb::new()
        .cache(cache_dir.path())
        .command()
        .arg("fetch")
        .current_dir(&t)
        .assert()
        .success()
        .stdout_eq(indoc! {r#"
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

    Scarb::quick_command()
        .arg("fetch")
        .current_dir(&t)
        .assert()
        .success();

    dep.child("src/lib.cairo")
        .write_str("fn hello() -> felt252 { 43 }")
        .unwrap();

    dep.git(["add", "."]);
    dep.git(["commit", "--amend", "-m", "amended"]);

    Scarb::quick_command()
        .arg("fetch")
        .current_dir(&t)
        .assert()
        .success()
        .stdout_eq(indoc! {r#"
        [..]  Updating git repository file://[..]/dep
        "#});
}

#[test]
fn force_push_with_cache() {
    let dep = gitx::new("dep", |t| {
        ProjectBuilder::start()
            .name("dep")
            .lib_cairo("fn hello() -> felt252 { 42 }")
            .build(&t)
    });
    let c = TempDir::new().unwrap();

    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello")
        .version("1.0.0")
        .dep("dep", &dep)
        .lib_cairo("fn world() -> felt252 { dep::hello() }")
        .build(&t);

    Scarb::new()
        .cache(c.path())
        .command()
        .arg("fetch")
        .current_dir(&t)
        .assert()
        .success();

    dep.child("src/lib.cairo")
        .write_str("fn hello() -> felt252 { 43 }")
        .unwrap();

    dep.git(["add", "."]);
    dep.git(["commit", "--amend", "-m", "amended"]);

    Scarb::new()
        .cache(c.path())
        .command()
        .arg("fetch")
        .current_dir(&t)
        .assert()
        .success()
        // Even though the locked commit is not accessible on remote no more, we do not update the
        // local git repository checkout, as the locked commit is still in the Scarb cache.
        .stdout_eq("");
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

    let metadata = Scarb::quick_command()
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

    Scarb::new()
        .cache(cache_dir.path())
        .command()
        .arg("fetch")
        .current_dir(&t)
        .assert()
        .success()
        .stdout_eq(indoc! {r#"
        [..]Updating git repository [..]dep1
        "#});

    Scarb::new()
        .cache(cache_dir.path())
        .command()
        .arg("fetch")
        .current_dir(&t)
        .assert()
        .success()
        .stdout_eq(Data::from("").raw());
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

    Scarb::new()
        .cache(cache_dir.path())
        .command()
        .arg("-v")
        .arg("fetch")
        .current_dir(&t)
        .assert()
        .success()
        .stdout_eq(indoc! {r#"
        [..]  Updating git repository file://[..]/dep1
        [..]Running git[EXE] fetch --verbose --force --update-head-ok [..]dep1[..] +HEAD:refs/remotes/origin/HEAD
        [..]Running git[EXE] clone --local --verbose --config 'core.autocrlf=false' --recurse-submodules [..].git[..] [..]
        [..]Running git[EXE] reset --hard [..]
        [..]Running git[EXE] submodule update --init --recursive --verbose
        "#});
    fs::remove_file(t.child("Scarb.lock")).unwrap();
    Scarb::new()
        .cache(cache_dir.path())
        .command()
        .arg("-v")
        .arg("fetch")
        .current_dir(&t)
        .assert()
        .success()
        .stdout_eq(indoc! {r#"
        [..]  Updating git repository file://[..]/dep1
        [..]Running git[EXE] fetch --verbose --force --update-head-ok [..]dep1[..] +HEAD:refs/remotes/origin/HEAD
        "#});
}
