use assert_fs::TempDir;
use assert_fs::assert::PathAssert;
use assert_fs::fixture::PathChild;
use indoc::indoc;
use scarb_test_support::command::Scarb;
use scarb_test_support::project_builder::ProjectBuilder;

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

    // Each component subdir is named `{component}-{hash}` and contains a file named `{component}`.
    let fp_dir = t.child("target/dev/.fingerprint").path().to_path_buf();
    let hello_fp = std::fs::read_dir(&fp_dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .find(|e| e.file_name().to_string_lossy().starts_with("hello-"))
        .unwrap()
        .path()
        .join("hello");
    let core_fp = std::fs::read_dir(&fp_dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .find(|e| e.file_name().to_string_lossy().starts_with("core-"))
        .unwrap()
        .path()
        .join("core");
    assert!(hello_fp.exists());
    assert!(core_fp.exists());

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

    // The freshness check creates hello-* as a side effect, but the fingerprint file inside it
    // must not be written when compilation fails. core-* is never created since freshness is only
    // checked for the main component.
    let fp_dir = t.child("target/dev/.fingerprint").path().to_path_buf();
    let hello_fp = std::fs::read_dir(&fp_dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .find(|e| e.file_name().to_string_lossy().starts_with("hello-"))
        .unwrap()
        .path()
        .join("hello");
    let core_subdir = std::fs::read_dir(&fp_dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .find(|e| e.file_name().to_string_lossy().starts_with("core-"));
    assert!(!hello_fp.exists());
    assert!(core_subdir.is_none());

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

    // Each component subdir is named `{component}-{hash}` and contains a file named `{component}`.
    let fp_dir = t.child("target/dev/.fingerprint").path().to_path_buf();
    let hello_fp = std::fs::read_dir(&fp_dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .find(|e| e.file_name().to_string_lossy().starts_with("hello-"))
        .unwrap()
        .path()
        .join("hello");
    let core_fp = std::fs::read_dir(&fp_dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .find(|e| e.file_name().to_string_lossy().starts_with("core-"))
        .unwrap()
        .path()
        .join("core");
    assert!(hello_fp.exists());
    assert!(core_fp.exists());

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
