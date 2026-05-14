use assert_fs::TempDir;
use assert_fs::assert::PathAssert;
use assert_fs::fixture::PathChild;
use indoc::indoc;
use predicates::prelude::PredicateBooleanExt;
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

    // Verify fingerprint cache was created
    t.child("target/dev/.fingerprint")
        .assert(predicates::path::exists());

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

    // Verify no fingerprint cache was created on error
    t.child("target/dev/.fingerprint")
        .assert(predicates::path::exists().not());

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

    // Verify fingerprint cache was created even with warnings
    t.child("target/dev/.fingerprint")
        .assert(predicates::path::exists());

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
