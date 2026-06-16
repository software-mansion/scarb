use assert_fs::TempDir;
use assert_fs::assert::PathAssert;
use assert_fs::fixture::PathChild;
use indoc::indoc;
use scarb_test_support::command::Scarb;
use scarb_test_support::project_builder::ProjectBuilder;
use scarb_test_support::workspace_builder::WorkspaceBuilder;
use std::fs;
use std::process::Command;

fn assert_package_fingerprint_presence(t: &TempDir, package_name: &str, should_exist: bool) {
    let fingerprint_dir = t.child("target/dev/.fingerprint");
    if !fingerprint_dir.path().exists() {
        assert!(
            !should_exist,
            "expected fingerprint directory to exist for package `{package_name}`"
        );
        return;
    }

    let mut found = false;
    for entry in fs::read_dir(fingerprint_dir.path()).unwrap().flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        if !name.starts_with(&format!("{package_name}-")) {
            continue;
        }

        if path.join(package_name).is_file() {
            found = true;
        }
    }

    assert_eq!(
        found, should_exist,
        "fingerprint presence mismatch for package `{package_name}`"
    );
}

#[test]
fn check_simple() {
    let cache_dir = TempDir::new().unwrap().child("c");

    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello")
        .version("0.1.0")
        .build(&t);

    Scarb::new()
        .cache(cache_dir.path())
        .command()
        .arg("check")
        .current_dir(&t)
        .assert()
        .stdout_eq(indoc! { r#"
        [..]Checking hello v0.1.0 ([..]Scarb.toml)
        [..]Finished checking `dev` profile target(s) in [..]
        "#
        })
        .success();

    cache_dir
        .child("registry/std")
        .assert(predicates::path::exists());
    cache_dir
        .child("CACHEDIR.TAG")
        .assert(predicates::path::exists());
}

#[test]
fn check_fail_with_syntax_error() {
    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello")
        .version("0.1.0")
        .lib_cairo("not_a_keyword")
        .build(&t);

    Scarb::quick_command()
        .arg("check")
        .current_dir(&t)
        .assert()
        .code(1)
        .stdout_eq(indoc! {r#"
                Checking hello v0.1.0 ([..]Scarb.toml)
            error[E1000]: Skipped tokens. Expected: Const/Enum/ExternFunction/ExternType/Function/Impl/InlineMacro/Module/Struct/Trait/TypeAlias/Use or an attribute.
             --> [..]/lib.cairo:1:14
            not_a_keyword
                         ^

            error: could not check `hello` due to [..] previous error
        "#});
}

#[test]
fn check_warnings_saved_for_main_package_after_dep_only_cache() {
    let t = TempDir::new().unwrap();
    let cache_dir = TempDir::new().unwrap();

    let pkg_deprecated = t.child("pkg_deprecated");
    ProjectBuilder::start()
        .name("pkg_deprecated")
        .lib_cairo(indoc! {r#"
            pub trait FooTrait {
                #[deprecated(
                    feature: "deprecated-foo",
                    note: "Use `FooTrait::bar` instead.",
                    since: "0.1.0",
                )]
                fn foo(self: u32) -> u32;
                fn bar(self: u32) -> u32;
            }
            pub impl FooImpl of FooTrait {
                fn foo(self: u32) -> u32 { self }
                fn bar(self: u32) -> u32 { self }
            }
        "#})
        .build(&pkg_deprecated);

    let pkg_user = t.child("pkg_user");
    ProjectBuilder::start()
        .name("pkg_user")
        .dep("pkg_deprecated", &pkg_deprecated)
        .lib_cairo(indoc! {r#"
            use pkg_deprecated::FooTrait;
            pub fn use_deprecated(x: u32) -> u32 {
                x.foo()
            }
        "#})
        .build(&pkg_user);

    let pkg_consumer = t.child("pkg_consumer");
    ProjectBuilder::start()
        .name("pkg_consumer")
        .dep("pkg_user", &pkg_user)
        .lib_cairo(indoc! {r#"
            use pkg_user::use_deprecated;
            pub fn consume(x: u32) -> u32 {
                use_deprecated(x)
            }
        "#})
        .build(&pkg_consumer);

    WorkspaceBuilder::start()
        .add_member("pkg_deprecated")
        .add_member("pkg_user")
        .add_member("pkg_consumer")
        .build(&t);

    // 1) Check dependent package first, which creates dep-only cache for pkg_user.
    Scarb::quick_command()
        .args(["check", "-p", "pkg_consumer"])
        .env("SCARB_CACHE", cache_dir.path())
        .current_dir(&t)
        .assert()
        .success();
    let cache_strings = |root: &assert_fs::fixture::ChildPath| {
        root.read_dir()
            .unwrap()
            .flatten()
            .map(|entry| entry.path())
            .filter(|path| {
                path.file_name()
                    .and_then(|name| name.to_str())
                    .map(|name| name.starts_with("pkg_user-") && name.ends_with(".bin"))
                    .unwrap_or(false)
            })
            .map(|path| {
                let output = Command::new("strings")
                    .arg(path)
                    .output()
                    .expect("failed to run strings on pkg_user check cache");
                assert!(output.status.success());
                String::from_utf8_lossy(&output.stdout).to_string()
            })
            .collect::<Vec<_>>()
    };
    let incremental_dir = t.child("target/dev/incremental");
    let strings_dep_only = cache_strings(&incremental_dir);
    assert!(
        !strings_dep_only.is_empty(),
        "pkg_user check cache file should exist after checking pkg_consumer"
    );

    // 2) Check pkg_user as main package; warning must be shown.
    Scarb::quick_command()
        .args(["check", "-p", "pkg_user"])
        .env("SCARB_CACHE", cache_dir.path())
        .current_dir(&t)
        .assert()
        .success()
        .stdout_eq(indoc! {r#"
        [..]Checking pkg_user v1.0.0 ([..]Scarb.toml)
        warn[E2066]: Usage of deprecated feature `"deprecated-foo"` with no `#[feature("deprecated-foo")]` attribute. Note: "Use `FooTrait::bar` instead."
         --> [..]lib.cairo:3:7
            x.foo()
              ^^^

        [..]Finished checking `dev` profile target(s) in [..]
        "#});
    let strings_main = cache_strings(&incremental_dir);
    assert!(
        strings_main != strings_dep_only,
        "pkg_user check cache textual snapshot should change after checking pkg_user as main package"
    );

    // 3) Check pkg_user again; warning should come from saved check cache.
    Scarb::quick_command()
        .args(["check", "-p", "pkg_user"])
        .env("SCARB_CACHE", cache_dir.path())
        .current_dir(&t)
        .assert()
        .success()
        .stdout_eq(indoc! {r#"
        [..]Checking pkg_user v1.0.0 ([..]Scarb.toml)
        warn[E2066]: Usage of deprecated feature `"deprecated-foo"` with no `#[feature("deprecated-foo")]` attribute. Note: "Use `FooTrait::bar` instead."
         --> [..]lib.cairo:3:7
            x.foo()
              ^^^

        [..]Finished checking `dev` profile target(s) in [..]
        "#});
}

#[test]
fn check_twice_success() {
    let cache_dir = TempDir::new().unwrap().child("c");

    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello")
        .version("0.1.0")
        .build(&t);

    // First check
    Scarb::new()
        .cache(cache_dir.path())
        .command()
        .arg("check")
        .current_dir(&t)
        .assert()
        .success()
        .stdout_eq(indoc! { r#"
        [..]Checking hello v0.1.0 ([..]Scarb.toml)
        [..]Finished checking `dev` profile target(s) in [..]
        "#
        });

    // Verify fingerprint cache file was created for `hello`.
    assert_package_fingerprint_presence(&t, "hello", true);

    // Second check - should use cached fingerprint
    Scarb::new()
        .cache(cache_dir.path())
        .command()
        .arg("check")
        .current_dir(&t)
        .assert()
        .success()
        .stdout_eq(indoc! { r#"
        [..]Checking hello v0.1.0 ([..]Scarb.toml)
        [..]Finished checking `dev` profile target(s) in [..]
        "#
        });
}

#[test]
fn check_twice_with_error() {
    let cache_dir = TempDir::new().unwrap().child("c");

    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello")
        .version("0.1.0")
        .lib_cairo("not_a_keyword")
        .build(&t);

    // First check - should fail
    Scarb::new()
        .cache(cache_dir.path())
        .command()
        .arg("check")
        .current_dir(&t)
        .assert()
        .code(1)
        .stdout_eq(indoc! { r#"
                Checking hello v0.1.0 ([..]Scarb.toml)
            error[E1000]: Skipped tokens. Expected: Const/Enum/ExternFunction/ExternType/Function/Impl/InlineMacro/Module/Struct/Trait/TypeAlias/Use or an attribute.
             --> [..]lib.cairo:1:14
            not_a_keyword
                         ^

            error: could not check `hello` due to [..] previous error
        "#
        });

    // Verify no `hello` fingerprint file was created on error.
    assert_package_fingerprint_presence(&t, "hello", false);

    // Second check - should fail with same error
    Scarb::new()
        .cache(cache_dir.path())
        .command()
        .arg("check")
        .current_dir(&t)
        .assert()
        .code(1)
        .stdout_eq(indoc! { r#"
                Checking hello v0.1.0 ([..]Scarb.toml)
            error[E1000]: Skipped tokens. Expected: Const/Enum/ExternFunction/ExternType/Function/Impl/InlineMacro/Module/Struct/Trait/TypeAlias/Use or an attribute.
             --> [..]lib.cairo:1:14
            not_a_keyword
                         ^

            error: could not check `hello` due to [..] previous error
        "#
        });
}

#[test]
fn check_twice_with_warning() {
    let cache_dir = TempDir::new().unwrap().child("c");

    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello")
        .version("0.1.0")
        .lib_cairo(indoc! {r#"
            pub mod math;
            use math::add;

            fn main() -> felt252 { 1 }
        "#})
        .src(
            "src/math.cairo",
            indoc! {r#"
                pub fn add(a: felt252, b: felt252) -> felt252 { a + b }
            "#},
        )
        .build(&t);

    // First check - should compile with warning about unused import
    Scarb::new()
        .cache(cache_dir.path())
        .command()
        .arg("check")
        .current_dir(&t)
        .assert()
        .success()
        .stdout_eq(indoc! { r#"
        [..]Checking hello v0.1.0 ([..]Scarb.toml)
        warn[E2100]: Unused import: `hello::add`
         --> [..]/lib.cairo:2:11
        use math::add;
                  ^^^

            Finished checking `dev` profile target(s) in [..]
        "#
        });

    // Verify `hello` fingerprint cache file was created even with warnings.
    assert_package_fingerprint_presence(&t, "hello", true);

    // Second check - should show same warning
    Scarb::new()
        .cache(cache_dir.path())
        .command()
        .arg("check")
        .current_dir(&t)
        .assert()
        .success()
        .stdout_eq(indoc! { r#"
        [..]Checking hello v0.1.0 ([..]Scarb.toml)
        warn[E2100]: Unused import: `hello::add`
         --> [..]/lib.cairo:2:11
        use math::add;
                  ^^^

            Finished checking `dev` profile target(s) in [..]
        "#
        });
}

#[test]
fn build_then_check_with_warning() {
    let cache_dir = TempDir::new().unwrap().child("c");

    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello")
        .version("0.1.0")
        .lib_cairo(indoc! {r#"
            pub mod math;
            use math::add;

            fn main() -> felt252 { 1 }
        "#})
        .src(
            "src/math.cairo",
            indoc! {r#"
                pub fn add(a: felt252, b: felt252) -> felt252 { a + b }
            "#},
        )
        .build(&t);

    // Build should show warning
    Scarb::new()
        .cache(cache_dir.path())
        .command()
        .arg("build")
        .current_dir(&t)
        .assert()
        .success()
        .stdout_eq(indoc! { r#"
        [..]Compiling hello v0.1.0 ([..]Scarb.toml)
        warn[E2100]: Unused import: `hello::add`
         --> [..]/lib.cairo:2:11
        use math::add;
                  ^^^

            Finished `dev` profile target(s) in [..]
        "#
        });

    // Check should show the same warning
    Scarb::new()
        .cache(cache_dir.path())
        .command()
        .arg("check")
        .current_dir(&t)
        .assert()
        .success()
        .stdout_eq(indoc! { r#"
        [..]Checking hello v0.1.0 ([..]Scarb.toml)
        warn[E2100]: Unused import: `hello::add`
         --> [..]/lib.cairo:2:11
        use math::add;
                  ^^^

            Finished checking `dev` profile target(s) in [..]
        "#
        });
}

#[test]
fn check_then_build() {
    let cache_dir = TempDir::new().unwrap().child("c");

    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello")
        .version("0.1.0")
        .build(&t);

    // Check should succeed
    Scarb::new()
        .cache(cache_dir.path())
        .command()
        .arg("check")
        .current_dir(&t)
        .assert()
        .success()
        .stdout_eq(indoc! { r#"
        [..]Checking hello v0.1.0 ([..]Scarb.toml)
        [..]Finished checking `dev` profile target(s) in [..]
        "#
        });

    assert_package_fingerprint_presence(&t, "hello", true);

    // Build should succeed after check
    Scarb::new()
        .cache(cache_dir.path())
        .command()
        .arg("build")
        .current_dir(&t)
        .assert()
        .success()
        .stdout_eq(indoc! { r#"
        [..]Compiling hello v0.1.0 ([..]Scarb.toml)
        [..]Finished `dev` profile target(s) in [..]
        "#
        });

    t.child("target/dev/hello.sierra.json")
        .assert(predicates::path::exists());
}

#[test]
fn check_then_build_then_check() {
    let cache_dir = TempDir::new().unwrap().child("c");

    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello")
        .version("0.1.0")
        .build(&t);

    // Check should succeed
    Scarb::new()
        .cache(cache_dir.path())
        .command()
        .arg("check")
        .current_dir(&t)
        .assert()
        .success()
        .stdout_eq(indoc! { r#"
        [..]Checking hello v0.1.0 ([..]Scarb.toml)
        [..]Finished checking `dev` profile target(s) in [..]
        "#
        });

    assert_package_fingerprint_presence(&t, "hello", true);

    // Build should succeed after check
    Scarb::new()
        .cache(cache_dir.path())
        .command()
        .arg("build")
        .current_dir(&t)
        .assert()
        .success()
        .stdout_eq(indoc! { r#"
        [..]Compiling hello v0.1.0 ([..]Scarb.toml)
        [..]Finished `dev` profile target(s) in [..]
        "#
        });

    t.child("target/dev/hello.sierra.json")
        .assert(predicates::path::exists());

    // Check should succeed after build
    Scarb::new()
        .cache(cache_dir.path())
        .command()
        .arg("check")
        .current_dir(&t)
        .assert()
        .success()
        .stdout_eq(indoc! { r#"
        [..]Checking hello v0.1.0 ([..]Scarb.toml)
        [..]Finished checking `dev` profile target(s) in [..]
        "#
        });
}
