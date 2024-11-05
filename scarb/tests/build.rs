use std::collections::HashMap;
use std::fs;

use assert_fs::prelude::*;
use assert_fs::TempDir;
use cairo_lang_sierra::program::{StatementIdx, VersionedProgram};
use cairo_lang_starknet_classes::contract_class::ContractClass;
use indoc::indoc;
use predicates::prelude::*;

use scarb_build_metadata::CAIRO_VERSION;
use scarb_metadata::Metadata;
use scarb_test_support::command::{CommandExt, Scarb};
use scarb_test_support::contracts::BALANCE_CONTRACT;
use scarb_test_support::fsx::ChildPathEx;
use scarb_test_support::project_builder::{Dep, DepBuilder, ProjectBuilder};
use scarb_test_support::workspace_builder::WorkspaceBuilder;
use serde::Deserialize;
use serde::Serialize;

#[test]
fn compile_simple() {
    // `TempDir::new` creates the directory, while `create_output_dir` does not mark directory as
    // ephemeral if it already exists.
    // Therefore, we use `.child` here to ensure that cache directory does not exist when running.
    let cache_dir = TempDir::new().unwrap().child("c");

    let t = TempDir::new().unwrap();
    ProjectBuilder::start().name("hello").build(&t);

    Scarb::quick_snapbox()
        .env("SCARB_CACHE", cache_dir.path())
        .arg("build")
        .current_dir(&t)
        .assert()
        .success();

    assert_eq!(t.child("target").files(), vec!["CACHEDIR.TAG", "dev"]);
    assert_eq!(t.child("target/dev").files(), vec!["hello.sierra.json"]);

    cache_dir
        .child("registry/std")
        .assert(predicates::path::exists());
    cache_dir
        .child("CACHEDIR.TAG")
        .assert(predicates::path::exists());
}

#[test]
fn quiet_output() {
    let t = TempDir::new().unwrap();
    ProjectBuilder::start().build(&t);

    Scarb::quick_snapbox()
        .args(["build", "-q"])
        .current_dir(&t)
        .assert()
        .success()
        .stdout_eq("");

    Scarb::quick_snapbox()
        .args(["--json", "-q", "build"])
        .current_dir(&t)
        .assert()
        .success()
        .stdout_eq("");
}

#[test]
fn compile_with_syntax_error() {
    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello")
        .version("0.1.0")
        .lib_cairo("not_a_keyword")
        .build(&t);

    Scarb::quick_snapbox()
        .arg("check")
        .current_dir(&t)
        .assert()
        .code(1)
        .stdout_matches(indoc! {r#"
                Checking hello v0.1.0 ([..]Scarb.toml)
            error: Skipped tokens. Expected: Const/Enum/ExternFunction/ExternType/Function/Impl/InlineMacro/Module/Struct/Trait/TypeAlias/Use or an attribute.
             --> [..]/lib.cairo:1:1
            not_a_keyword
            ^***********^

            error: could not check `hello` due to previous error
        "#});
}

#[test]
fn compile_with_syntax_error_json() {
    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello")
        .version("0.1.0")
        .lib_cairo("not_a_keyword")
        .build(&t);

    Scarb::quick_snapbox()
        .arg("--json")
        .arg("check")
        .current_dir(&t)
        .assert()
        .code(1)
        .stdout_matches(indoc! {r#"
            {"status":"checking","message":"hello v0.1.0 ([..]Scarb.toml)"}
            {"type":"error","message":"Skipped tokens. Expected: Const/Enum/ExternFunction/ExternType/Function/Impl/InlineMacro/Module/Struct/Trait/TypeAlias/Use or an attribute./n --> [..]/lib.cairo:1:1/nnot_a_keyword/n^***********^/n"}
            {"type":"error","message":"could not check `hello` due to previous error"}
        "#});
}

#[test]
fn compile_without_manifest() {
    let t = TempDir::new().unwrap();
    let cause = fs::read(t.child("Scarb.toml")).unwrap_err();
    Scarb::quick_snapbox()
        .arg("build")
        .current_dir(&t)
        .assert()
        .code(1)
        .stdout_matches(format!(
            "\
error: failed to read manifest at: [..]/Scarb.toml

Caused by:
    {cause}
"
        ));
}

#[test]
#[cfg(target_os = "linux")]
fn compile_with_lowercase_scarb_toml() {
    let t = TempDir::new().unwrap();
    t.child("scarb.toml")
        .write_str(
            r#"
            [package]
            name = "hello"
            version = "0.1.0"
            "#,
        )
        .unwrap();
    let cause = fs::read(t.child("Scarb.toml")).unwrap_err();
    Scarb::quick_snapbox()
        .arg("build")
        .current_dir(&t)
        .assert()
        .code(1)
        .stdout_matches(format!(
            "\
error: failed to read manifest at: [..]/Scarb.toml

Caused by:
    {cause}
"
        ));
}

#[test]
fn compile_with_manifest_not_a_file() {
    let t = TempDir::new().unwrap();
    t.child("Scarb.toml").create_dir_all().unwrap();
    let cause = fs::read(t.child("Scarb.toml")).unwrap_err();
    Scarb::quick_snapbox()
        .arg("build")
        .current_dir(&t)
        .assert()
        .code(1)
        .stdout_matches(format!(
            "\
error: failed to read manifest at: [..]/Scarb.toml

Caused by:
    {cause}
"
        ));
}

#[test]
fn compile_with_invalid_empty_name() {
    let t = TempDir::new().unwrap();
    t.child("Scarb.toml")
        .write_str(
            r#"
            [package]
            name = ""
            version = "0.1.0"
            "#,
        )
        .unwrap();
    Scarb::quick_snapbox()
        .arg("build")
        .current_dir(&t)
        .assert()
        .code(1)
        .stdout_matches(indoc! {r#"
            error: failed to parse manifest at: [..]/Scarb.toml

            Caused by:
                TOML parse error at line 3, column 20
                  |
                3 |             name = ""
                  |                    ^^
                empty string cannot be used as package name
        "#});
}

#[test]
fn compile_with_invalid_version() {
    let t = TempDir::new().unwrap();
    t.child("Scarb.toml")
        .write_str(
            r#"
            [package]
            name = "foo"
            version = "y"
            "#,
        )
        .unwrap();
    Scarb::quick_snapbox()
        .arg("build")
        .current_dir(&t)
        .assert()
        .code(1)
        .stdout_matches(indoc! {r#"
            error: failed to parse manifest at: [..]/Scarb.toml

            Caused by:
                TOML parse error at line 4, column 23
                  |
                4 |             version = "y"
                  |                       ^^^
                unexpected character 'y' while parsing major version number
        "#});
}

#[test]
fn compile_with_invalid_cairo_version() {
    let t = TempDir::new().unwrap();
    t.child("Scarb.toml")
        .write_str(
            r#"
            [package]
            name = "foo"
            version = "0.1.0"
            cairo-version = "f"
            "#,
        )
        .unwrap();
    Scarb::quick_snapbox()
        .arg("build")
        .current_dir(&t)
        .assert()
        .code(1)
        .stdout_matches(indoc! {r#"
            error: failed to parse manifest at: [..]/Scarb.toml

            Caused by:
                TOML parse error at line 5, column 29
                  |
                5 |             cairo-version = "f"
                  |                             ^^^
                unexpected character 'f' while parsing major version number
        "#});
}

#[test]
fn compile_with_incompatible_cairo_version() {
    let t = TempDir::new().unwrap();
    t.child("Scarb.toml")
        .write_str(
            r#"
            [package]
            name = "hello"
            version = "0.1.0"
            edition = "2023_01"
            cairo-version = "33.33.0"
            "#,
        )
        .unwrap();
    Scarb::quick_snapbox()
        .arg("check")
        .current_dir(&t)
        .assert()
        .code(1)
        .stdout_matches(indoc! {r#"
            error: the required Cairo version of package hello is not compatible with current version
            Cairo version required: ^33.33.0
            Cairo version of Scarb: [..]

            error: the required Cairo version of each package must match the current Cairo version
            help: pass `--ignore-cairo-version` to ignore Cairo version mismatch
        "#});
}

#[test]
fn compile_ignore_cairo_version() {
    let t = TempDir::new().unwrap();
    t.child("Scarb.toml")
        .write_str(
            r#"
            [package]
            name = "hello"
            version = "0.1.0"
            cairo-version = "33.33.0"
            "#,
        )
        .unwrap();
    t.child("src/lib.cairo")
        .write_str("fn example() -> felt252 { 42 }")
        .unwrap();
    Scarb::quick_snapbox()
        .args(["check", "--ignore-cairo-version"])
        .current_dir(&t)
        .assert()
        .success()
        .stdout_matches(indoc! {r#"
            warn: `edition` field not set in `[package]` section for package `hello`
            warn: the required Cairo version of package hello is not compatible with current version
            Cairo version required: ^33.33.0
            Cairo version of Scarb: [..]

            [..] Checking hello v0.1.0 ([..]Scarb.toml)
            [..] Finished checking `dev` profile target(s) in [..]
        "#});
}

#[test]
fn compile_with_compatible_cairo_version() {
    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello")
        .version("0.1.0")
        .cairo_version(CAIRO_VERSION)
        .build(&t);

    Scarb::quick_snapbox()
        .args(["build"])
        .current_dir(&t)
        .assert()
        .success();
}

#[test]
fn compile_with_invalid_non_numeric_dep_version() {
    let t = TempDir::new().unwrap();
    t.child("Scarb.toml")
        .write_str(
            r#"
            [package]
            name = "hello"
            version = "0.1.0"

            [dependencies]
            moo = "y"
            "#,
        )
        .unwrap();
    Scarb::quick_snapbox()
        .arg("build")
        .current_dir(&t)
        .assert()
        .code(1)
        .stdout_matches(indoc! {r#"
            error: failed to parse manifest at: [..]/Scarb.toml

            Caused by:
                TOML parse error at line 7, column 19
                  |
                7 |             moo = "y"
                  |                   ^^^
                data did not match any variant of untagged enum TomlDependency
        "#});
}

#[test]
fn compile_with_unset_edition() {
    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello")
        .version("0.1.0")
        .no_edition()
        .build(&t);

    Scarb::quick_snapbox()
        .arg("build")
        .current_dir(&t)
        .assert()
        .success()
        .stdout_matches(indoc! {r#"
            warn: `edition` field not set in `[package]` section for package `hello`
            [..] Compiling hello v0.1.0 ([..]Scarb.toml)
            [..]  Finished `dev` profile target(s) in [..]
        "#});
}

#[test]
fn compile_multiple_packages() {
    let t = TempDir::new().unwrap();

    t.child("Scarb.toml")
        .write_str(
            r#"
            [package]
            name = "fib"
            version = "1.0.0"
            edition = "2023_01"

            [dependencies]
            decrement = { path = "decrement" }
            "#,
        )
        .unwrap();

    t.child("src/lib.cairo")
        .write_str(
            r#"
            mod sum_two;

            fn fib(a: felt252, b: felt252, n: felt252) -> felt252 {
                match n {
                    0 => a,
                    _ => fib(b, sum_two::sum_two(a, b), decrement::decrement(n)),
                }
            }
            "#,
        )
        .unwrap();

    t.child("src/sum_two.cairo")
        .write_str(r#"fn sum_two(a: felt252, b: felt252) -> felt252 { a + b }"#)
        .unwrap();

    t.child("decrement/Scarb.toml")
        .write_str(
            r#"
            [package]
            name = "decrement"
            version = "1.0.0"
            edition = "2023_01"
            "#,
        )
        .unwrap();

    t.child("decrement/src/lib.cairo")
        .write_str(
            r#"
            fn decrement(x: felt252) -> felt252 { x - 1 }
            "#,
        )
        .unwrap();

    Scarb::quick_snapbox()
        .arg("build")
        .current_dir(&t)
        .assert()
        .success()
        .stdout_matches(indoc! {r#"
            [..] Compiling fib v1.0.0 ([..]Scarb.toml)
            [..]  Finished `dev` profile target(s) in [..]
        "#});

    t.child("target/dev/fib.sierra.json")
        .assert(predicates::str::is_empty().not());
}

#[test]
fn compile_with_nested_deps() {
    let t = TempDir::new().unwrap();

    t.child("Scarb.toml")
        .write_str(
            r#"
            [package]
            name = "x"
            version = "1.0.0"
            edition = "2023_01"

            [dependencies]
            y = { path = "y" }
            "#,
        )
        .unwrap();

    t.child("src/lib.cairo")
        .write_str(r"fn f() -> felt252 { y::f() }")
        .unwrap();

    t.child("y/Scarb.toml")
        .write_str(
            r#"
            [package]
            name = "y"
            version = "1.0.0"
            edition = "2023_01"

            [dependencies]
            q = { path = "../q" }
            z = { path = "../z" }
            "#,
        )
        .unwrap();

    t.child("y/src/lib.cairo")
        .write_str(r"fn f() -> felt252 { z::f() }")
        .unwrap();

    t.child("z/Scarb.toml")
        .write_str(
            r#"
            [package]
            name = "z"
            version = "1.0.0"
            edition = "2023_01"

            [dependencies]
            q = { path = "../q" }
            "#,
        )
        .unwrap();

    t.child("z/src/lib.cairo")
        .write_str(r"fn f() -> felt252 { q::f() }")
        .unwrap();

    t.child("q/Scarb.toml")
        .write_str(
            r#"
            [package]
            name = "q"
            version = "1.0.0"
            edition = "2023_01"
            "#,
        )
        .unwrap();

    t.child("q/src/lib.cairo")
        .write_str(r"fn f() -> felt252 { 42 }")
        .unwrap();

    Scarb::quick_snapbox()
        .arg("build")
        .current_dir(&t)
        .assert()
        .success()
        .stdout_matches(indoc! {r#"
            [..] Compiling x v1.0.0 ([..]Scarb.toml)
            [..]  Finished `dev` profile target(s) in [..]
        "#});

    t.child("target/dev/x.sierra.json")
        .assert(predicates::str::is_empty().not());
}

#[test]
fn override_target_dir() {
    let target_dir = TempDir::new().unwrap();

    let t = TempDir::new().unwrap();
    ProjectBuilder::start().name("hello").build(&t);

    Scarb::quick_snapbox()
        .arg("--target-dir")
        .arg(target_dir.path())
        .arg("build")
        .current_dir(&t)
        .assert()
        .success();

    t.child("target").assert(predicates::path::exists().not());
    target_dir
        .child("dev/hello.sierra.json")
        .assert(predicates::path::exists());
}

#[test]
fn sierra_replace_ids() {
    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello")
        .lib_cairo("fn example() -> felt252 { 42 }")
        .manifest_extra(
            r#"
            [cairo]
            sierra-replace-ids = true
            "#,
        )
        .build(&t);

    Scarb::quick_snapbox()
        .arg("build")
        .current_dir(&t)
        .assert()
        .success();

    t.child("target/dev/hello.sierra.json").assert(
        predicates::str::contains(r#""debug_name":"hello::example""#)
            .and(predicates::str::contains(
                r#""debug_name":"Const<felt252, 42>""#,
            ))
            .and(predicates::str::contains(
                r#""debug_name":"store_temp<felt252>""#,
            ))
            .and(predicates::str::contains(r#""debug_name":null"#)),
    );
}

#[test]
fn workspace_as_dep() {
    let t = TempDir::new().unwrap();

    let first_t = t.child("first_workspace");
    let pkg1 = first_t.child("first");
    ProjectBuilder::start().name("first").build(&pkg1);
    let pkg2 = first_t.child("second");
    ProjectBuilder::start()
        .name("second")
        .dep("first", Dep.path("../first"))
        .lib_cairo(indoc! {r#"
        fn fib(a: felt252, b: felt252, n: felt252) -> felt252 {
            match n {
                0 => a,
                _ => fib(b, a + b, n - 1),
            }
        }

        #[cfg(test)]
        mod tests {
            use super::fib;

            #[test]
            fn it_works() {
                assert(fib(0, 1, 16) == 987, 'it works!');
            }
        }
        "#})
        .build(&pkg2);
    WorkspaceBuilder::start()
        .add_member("first")
        .add_member("second")
        .build(&first_t);

    let second_t = t.child("second_workspace");
    let pkg1 = second_t.child("third");
    ProjectBuilder::start()
        .name("third")
        .dep("first", Dep.path("../../first_workspace"))
        .dep("second", Dep.path("../../first_workspace"))
        .lib_cairo(indoc! {r#"
            use second::fib;
            fn example() -> felt252 { 42 }

            fn hello() -> felt252 {
                fib(0, 1, 16)
            }
        "#})
        .build(&pkg1);
    let pkg2 = second_t.child("fourth");
    ProjectBuilder::start()
        .name("fourth")
        .dep("third", Dep.path("../third"))
        .build(&pkg2);
    WorkspaceBuilder::start()
        .add_member("third")
        .add_member("fourth")
        .manifest_extra(
            r#"
            [cairo]
            sierra-replace-ids = true
            "#,
        )
        .build(&second_t);

    Scarb::quick_snapbox()
        .arg("build")
        .current_dir(&second_t)
        .assert()
        .success();

    assert_eq!(
        second_t.child("target").files(),
        vec!["CACHEDIR.TAG", "dev"]
    );
    assert_eq!(
        second_t.child("target/dev").files(),
        vec!["fourth.sierra.json", "third.sierra.json"]
    );
    second_t.child("target/dev/third.sierra.json").assert(
        predicates::str::contains(r#""debug_name":"third::example""#)
            .and(predicates::str::contains(r#""debug_name":"third::hello""#)),
    );
    second_t
        .child("target/dev/third.sierra.json")
        .assert(predicates::str::contains(r#""debug_name":"second::fib""#));
}

#[test]
fn can_define_edition() {
    let code = indoc! {r#"
        fn example() -> Nullable<felt252> { null() }
    "#};
    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .lib_cairo(code)
        .edition("2023_01")
        .build(&t);

    Scarb::quick_snapbox()
        .arg("build")
        .current_dir(&t)
        .assert()
        .success();

    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .lib_cairo(code)
        .edition("2023_10")
        .build(&t);

    Scarb::quick_snapbox()
        .arg("build")
        .current_dir(&t)
        .assert()
        .failure();
}

#[test]
fn edition_must_exist() {
    let t = TempDir::new().unwrap();
    ProjectBuilder::start().edition("2021").build(&t);

    Scarb::quick_snapbox()
        .arg("fetch")
        .current_dir(&t)
        .assert()
        .failure()
        .stdout_matches(indoc! {r#"
             error: failed to parse manifest at: [..]/Scarb.toml

             Caused by:
                 TOML parse error at line 4, column 11
                   |
                 4 | edition = "2021"
                   |           ^^^^^^
                 unknown variant `2021`, expected one of `2023_01`, `2023_10`, `2023_11`, `2024_07`
        "#});
}

#[test]
fn dev_dep_used_outside_tests() {
    let t = TempDir::new().unwrap();
    let q = t.child("q");
    ProjectBuilder::start()
        .name("q")
        .lib_cairo("fn dev_dep_function() -> felt252 { 42 }")
        .build(&q);
    ProjectBuilder::start()
        .name("x")
        .dev_dep("q", &q)
        .lib_cairo(indoc! {r#"
            use q::dev_dep_function;

            fn not_working() {
                dev_dep_function();
            }
        "#})
        .build(&t);

    Scarb::quick_snapbox()
        .arg("build")
        .current_dir(&t)
        .assert()
        .failure()
        .stdout_matches(indoc! {r#"
            [..] Compiling x v1.0.0 ([..])
            error: Identifier not found.
             --> [..]/src/lib.cairo[..]
            use q::dev_dep_function;
                ^

            error: could not compile `x` due to previous error
        "#});
}

#[test]
fn dev_dep_inside_test() {
    let t = TempDir::new().unwrap();
    let q = t.child("q");
    ProjectBuilder::start()
        .name("q")
        .lib_cairo("fn dev_dep_function() -> felt252 { 42 }")
        .build(&q);
    ProjectBuilder::start()
        .name("x")
        .dev_dep("q", &q)
        .lib_cairo(indoc! {r#"
            #[cfg(test)]
            mod tests {
                use q::dev_dep_function;

                fn it_works() {
                    dev_dep_function();
                }
            }
        "#})
        .build(&t);

    Scarb::quick_snapbox()
        .arg("build")
        .current_dir(&t)
        .assert()
        .success()
        .stdout_matches(indoc! {r#"
            [..] Compiling x v1.0.0 ([..])
            [..]  Finished `dev` profile target(s) in [..]
        "#});
}

#[test]
fn build_test_without_compiling_tests_from_dependencies() {
    let t = TempDir::new().unwrap();
    let q = t.child("q");
    ProjectBuilder::start()
        .name("q")
        .lib_cairo(indoc! {r#"
            fn dev_dep_function() -> felt252 { 42 }

            #[cfg(test)]
            mod tests {
                use missing::func;
            }
        "#})
        .build(&q);
    ProjectBuilder::start()
        .name("x")
        .dev_dep("q", &q)
        .lib_cairo(indoc! {r#"
            #[cfg(test)]
            mod tests {
                use q::dev_dep_function;

                fn it_works() {
                    dev_dep_function();
                }
            }
        "#})
        .build(&t);

    Scarb::quick_snapbox()
        .arg("build")
        .arg("--test")
        .current_dir(&t)
        .assert()
        .success()
        .stdout_matches(indoc! {r#"
            [..] Compiling test(x_unittest) x v1.0.0 ([..])
            [..]  Finished `dev` profile target(s) in [..]
        "#});
}

#[test]
fn warnings_allowed_by_default() {
    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .lib_cairo(indoc! {r#"
        fn hello() -> felt252 {
            let a = 41;
            let b = 42;
            b
        }
    "#})
        .build(&t);

    Scarb::quick_snapbox()
        .arg("build")
        .current_dir(&t)
        .assert()
        .success()
        .stdout_matches(indoc! {r#"
        [..] Compiling [..] v1.0.0 ([..]Scarb.toml)
        warn[E0001]: Unused variable. Consider ignoring by prefixing with `_`.
         --> [..]lib.cairo:2:9
            let a = 41;
                ^

            Finished `dev` profile target(s) in [..]
        "#});
}

#[test]
fn warnings_can_be_disallowed() {
    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .lib_cairo(indoc! {r#"
        fn hello() -> felt252 {
            let a = 41;
            let b = 42;
            b
        }
        "#})
        .manifest_extra(indoc! {r#"
        [cairo]
        allow-warnings = false
        "#})
        .build(&t);

    Scarb::quick_snapbox()
        .arg("build")
        .current_dir(&t)
        .assert()
        .failure()
        .stdout_matches(indoc! {r#"
        [..] Compiling [..] v1.0.0 ([..]Scarb.toml)
        warn[E0001]: Unused variable. Consider ignoring by prefixing with `_`.
         --> [..]lib.cairo:2:9
            let a = 41;
                ^

        error: could not compile [..] due to previous error
        "#});
}

#[test]
fn does_show_errors_from_deps() {
    let t = TempDir::new().unwrap();
    let first = t.child("first");
    let second = t.child("second");
    ProjectBuilder::start()
        .name("first")
        .lib_cairo(indoc! {r#"
        fn hello() -> felt252 {
            ;
            12
        }
        "#})
        .manifest_extra(indoc! {r#"
        [cairo]
        allow-warnings = false
        "#})
        .build(&first);
    ProjectBuilder::start()
        .name("second")
        .lib_cairo(indoc! {r#"
            fn hello() -> felt252 { 42 }
        "#})
        .dep("first", &first)
        .manifest_extra(indoc! {r#"
        [cairo]
        allow-warnings = false
        "#})
        .build(&second);
    Scarb::quick_snapbox()
        .arg("build")
        .current_dir(&second)
        .assert()
        .failure()
        .stdout_matches(indoc! {r#"
            [..]Compiling second v1.0.0 ([..]Scarb.toml)
            error: Skipped tokens. Expected: statement.
             --> [..]lib.cairo:2:5
                ;
                ^
            
            error: could not compile `second` due to previous error
        "#});
}

#[test]
fn does_not_show_warnings_from_deps() {
    let t = TempDir::new().unwrap();
    let first = t.child("first");
    let second = t.child("second");
    ProjectBuilder::start()
        .name("first")
        .lib_cairo(indoc! {r#"
        fn hello() -> felt252 {
            let a = 41;
            let b = 42;
            b
        }
        "#})
        .manifest_extra(indoc! {r#"
        [cairo]
        allow-warnings = false
        "#})
        .build(&first);
    ProjectBuilder::start()
        .name("second")
        .lib_cairo(indoc! {r#"
            fn hello() -> felt252 { 42 }
        "#})
        .dep("first", &first)
        .manifest_extra(indoc! {r#"
        [cairo]
        allow-warnings = false
        "#})
        .build(&second);
    Scarb::quick_snapbox()
        .arg("build")
        .current_dir(&first)
        .assert()
        .failure()
        .stdout_matches(indoc! {r#"
        [..] Compiling [..] v1.0.0 ([..]Scarb.toml)
        warn[E0001]: Unused variable. Consider ignoring by prefixing with `_`.
         --> [..]lib.cairo:2:9
            let a = 41;
                ^

        error: could not compile [..] due to previous error
        "#});
    Scarb::quick_snapbox()
        .arg("build")
        .current_dir(&second)
        .assert()
        .success();
    Scarb::quick_snapbox()
        .arg("build")
        .arg("--test")
        .current_dir(&second)
        .assert()
        .success();
}

#[test]
fn error_codes_shown_in_json_output() {
    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .lib_cairo(indoc! {r#"
        fn hello() -> felt252 {
            let a = 41;
            let b = 42;
            b
        }
    "#})
        .build(&t);

    Scarb::quick_snapbox()
        .arg("--json")
        .arg("build")
        .current_dir(&t)
        .assert()
        .success()
        .stdout_matches(indoc! {r#"
            {"status":"compiling","message":"[..] v1.0.0 ([..]Scarb.toml)"}
            {"type":"warn","message":"Unused variable. Consider ignoring by prefixing with `_`./n --> [..]lib.cairo:2:9/n    let a = 41;/n        ^/n","code":"E0001"}
            {"status":"finished","message":"`dev` profile target(s) in [..]"}
        "#});
}

#[test]
fn can_compile_no_core_package() {
    let t = TempDir::new().unwrap();
    // Find path to corelib.
    ProjectBuilder::start().name("hello").build(&t);
    let metadata = Scarb::quick_snapbox()
        .args(["--json", "metadata", "--format-version", "1"])
        .current_dir(&t)
        .stdout_json::<Metadata>();
    let core = metadata.packages.iter().find(|p| p.name == "core").unwrap();
    let core = core.root.clone();
    // Compile corelib.
    Scarb::quick_snapbox()
        .arg("build")
        .current_dir(core)
        .assert()
        .success();
}

#[test]
fn gas_enabled_by_default() {
    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello")
        .lib_cairo(indoc! {r#"
            fn main() -> u32 {
                fib(16)
            }

            fn fib(mut n: u32) -> u32 {
                let mut a: u32 = 0;
                let mut b: u32 = 1;
                while n != 0 {
                    n = n - 1;
                    let temp = b;
                    b = a + b;
                    a = temp;
                };
                a
            }
        "#})
        .build(&t);
    Scarb::quick_snapbox()
        .arg("build")
        .current_dir(&t)
        .assert()
        .success();
    t.child("target/dev/hello.sierra.json")
        .assert(predicates::str::contains(r#""withdraw_gas""#));
}

#[test]
fn can_disable_gas() {
    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello")
        // Note: Compiling to CASM will fail on gas validation in `calc_metadata`
        // unless `calc_metadata_ap_change_only` is used.
        .lib_cairo(indoc! {r#"
            fn foo(mut shape: Span<usize>) -> usize {
                let mut result: usize = 1;

                loop {
                    match shape.pop_front() {
                        Option::Some(item) => { result *= *item; },
                        Option::None => { break; }
                    };
                };

                result
            }

            fn main() -> usize {
                foo(array![1, 2].span())
            }
        "#})
        .manifest_extra(indoc! {r#"
            [lib]
            casm = true

            [cairo]
            enable-gas = false
        "#})
        .build(&t);
    Scarb::quick_snapbox()
        .arg("build")
        .current_dir(&t)
        .assert()
        .success();
    t.child("target/dev/hello.sierra.json")
        .assert(predicates::str::contains(r#""withdraw_gas""#).not());
}

#[test]
fn cannot_disable_gas_for_starknet_contract() {
    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello")
        .dep_starknet()
        .lib_cairo(BALANCE_CONTRACT)
        .manifest_extra(indoc! {r#"
            [[target.starknet-contract]]

            [cairo]
            enable-gas = false
        "#})
        .build(&t);
    Scarb::quick_snapbox()
        .arg("build")
        .current_dir(&t)
        .assert()
        .failure()
        .stdout_matches(indoc! {r#"
            [..]Compiling hello v1.0.0 ([..]Scarb.toml)
            error: the target starknet contract compilation requires gas to be enabled
            error: could not compile `hello` due to previous error
        "#});
}

#[test]
fn add_statements_functions_debug_info() {
    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello")
        .lib_cairo(indoc! {r##"
            #[starknet::interface]
            pub trait IHelloStarknet<TContractState> {
                fn increase_balance(ref self: TContractState, amount: felt252);
                fn get_balance(self: @TContractState) -> felt252;
            }

            #[starknet::contract]
            mod HelloStarknet {
                #[storage]
                struct Storage {
                    balance: felt252,
                }

                #[abi(embed_v0)]
                impl HelloStarknetImpl of super::IHelloStarknet<ContractState> {
                    fn increase_balance(ref self: ContractState, amount: felt252) {
                        assert(amount != 0, 'Amount cannot be 0');
                        self.balance.write(self.balance.read() + amount);
                    }

                    fn get_balance(self: @ContractState) -> felt252 {
                        self.balance.read()
                    }
                }
            }

            fn foo(mut shape: Span<usize>) -> usize {
                let mut result: usize = 1;

                loop {
                    match shape.pop_front() {
                        Option::Some(item) => { result *= *item; },
                        Option::None => { break; }
                    };
                };

                result
            }

            fn main() -> usize {
                foo(array![1, 2].span())
            }
        "##})
        .manifest_extra(indoc! {r#"
            [lib]
            casm = true

            [[target.starknet-contract]]
            casm = true

            [cairo]
            unstable-add-statements-functions-debug-info = true
        "#})
        .dep_starknet()
        .build(&t);
    Scarb::quick_snapbox()
        .arg("build")
        .current_dir(&t)
        .assert()
        .success();

    let lib_sierra_string = t.child("target/dev/hello.sierra.json").read_to_string();
    let contract_sierra_string = t
        .child("target/dev/hello_HelloStarknet.contract_class.json")
        .read_to_string();
    let lib_sierra = serde_json::from_str::<VersionedProgram>(&lib_sierra_string).unwrap();
    let contract_sierra = serde_json::from_str::<ContractClass>(&contract_sierra_string).unwrap();

    let debug_info = lib_sierra
        .into_v1()
        .unwrap()
        .debug_info
        .expect("Expected debug info to exist");
    let mappings = debug_info
        .annotations
        .get("github.com/software-mansion/cairo-profiler")
        .expect("Expected cairo-profiler annotations to exist")
        .get("statements_functions")
        .expect("Expected statements_functions info to exist");
    assert!(
        serde_json::from_value::<HashMap<StatementIdx, Vec<String>>>(mappings.clone()).is_ok(),
        "Expected statements_functions info to be a map"
    );

    let debug_info = contract_sierra
        .sierra_program_debug_info
        .expect("Expected debug info to exist");
    let mappings = debug_info
        .annotations
        .get("github.com/software-mansion/cairo-profiler")
        .expect("Expected cairo-profiler annotations to exist")
        .get("statements_functions")
        .expect("Expected statements_functions info to exist");
    assert!(
        serde_json::from_value::<HashMap<StatementIdx, Vec<String>>>(mappings.clone()).is_ok(),
        "Expected statements_functions info to be a map"
    );
}

#[derive(Serialize, Deserialize)]
pub struct SourceCodeLocation {
    pub line: usize,
    pub col: usize,
}

#[derive(Serialize, Deserialize)]
pub struct SourceCodeSpan {
    pub start: SourceCodeLocation,
    pub end: SourceCodeLocation,
}

#[test]
fn add_statements_code_locations_debug_info() {
    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello")
        .lib_cairo(indoc! {r##"
            #[starknet::interface]
            pub trait IHelloStarknet<TContractState> {
                fn increase_balance(ref self: TContractState, amount: felt252);
                fn get_balance(self: @TContractState) -> felt252;
            }

            #[starknet::contract]
            mod HelloStarknet {
                #[storage]
                struct Storage {
                    balance: felt252,
                }

                #[abi(embed_v0)]
                impl HelloStarknetImpl of super::IHelloStarknet<ContractState> {
                    fn increase_balance(ref self: ContractState, amount: felt252) {
                        assert(amount != 0, 'Amount cannot be 0');
                        self.balance.write(self.balance.read() + amount);
                    }

                    fn get_balance(self: @ContractState) -> felt252 {
                        self.balance.read()
                    }
                }
            }

            fn foo(mut shape: Span<usize>) -> usize {
                let mut result: usize = 1;

                loop {
                    match shape.pop_front() {
                        Option::Some(item) => { result *= *item; },
                        Option::None => { break; }
                    };
                };

                result
            }

            fn main() -> usize {
                foo(array![1, 2].span())
            }
        "##})
        .manifest_extra(indoc! {r#"
            [lib]
            casm = true

            [[target.starknet-contract]]
            casm = true

            [cairo]
            unstable-add-statements-code-locations-debug-info = true
        "#})
        .dep_starknet()
        .build(&t);
    Scarb::quick_snapbox()
        .arg("build")
        .current_dir(&t)
        .assert()
        .success();

    let lib_sierra_string = t.child("target/dev/hello.sierra.json").read_to_string();
    let contract_sierra_string = t
        .child("target/dev/hello_HelloStarknet.contract_class.json")
        .read_to_string();
    let lib_sierra = serde_json::from_str::<VersionedProgram>(&lib_sierra_string).unwrap();
    let contract_sierra = serde_json::from_str::<ContractClass>(&contract_sierra_string).unwrap();

    let debug_info = lib_sierra
        .into_v1()
        .unwrap()
        .debug_info
        .expect("Expected debug info to exist");
    let mappings = debug_info
        .annotations
        .get("github.com/software-mansion/cairo-coverage")
        .expect("Expected cairo-coverage annotations to exist")
        .get("statements_code_locations")
        .expect("Expected statements_code_locations info to exist");
    assert!(
        serde_json::from_value::<HashMap<StatementIdx, Vec<(String, SourceCodeSpan)>>>(
            mappings.clone()
        )
        .is_ok(),
        "Expected statements_code_locations info to be a map"
    );

    let debug_info = contract_sierra
        .sierra_program_debug_info
        .expect("Expected debug info to exist");
    let mappings = debug_info
        .annotations
        .get("github.com/software-mansion/cairo-coverage")
        .expect("Expected cairo-coverage annotations to exist")
        .get("statements_code_locations")
        .expect("Expected statements_code_locations info to exist");
    assert!(
        serde_json::from_value::<HashMap<StatementIdx, Vec<(String, SourceCodeSpan)>>>(
            mappings.clone()
        )
        .is_ok(),
        "Expected statements_code_locations info to be a map"
    );
}

#[test]
fn add_statements_functions_debug_info_to_tests() {
    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello")
        .lib_cairo(indoc! {r##"
            #[starknet::interface]
            pub trait IHelloStarknet<TContractState> {
                fn increase_balance(ref self: TContractState, amount: felt252);
                fn get_balance(self: @TContractState) -> felt252;
            }

            #[starknet::contract]
            mod HelloStarknet {
                #[storage]
                struct Storage {
                    balance: felt252,
                }

                #[abi(embed_v0)]
                impl HelloStarknetImpl of super::IHelloStarknet<ContractState> {
                    fn increase_balance(ref self: ContractState, amount: felt252) {
                        assert(amount != 0, 'Amount cannot be 0');
                        self.balance.write(self.balance.read() + amount);
                    }

                    fn get_balance(self: @ContractState) -> felt252 {
                        self.balance.read()
                    }
                }
            }

            fn foo(mut shape: Span<usize>) -> usize {
                let mut result: usize = 1;

                loop {
                    match shape.pop_front() {
                        Option::Some(item) => { result *= *item; },
                        Option::None => { break; }
                    };
                };

                result
            }

            fn main() -> usize {
                foo(array![1, 2].span())
            }
        "##})
        .manifest_extra(indoc! {r#"
            [[target.starknet-contract]]

            [cairo]
            unstable-add-statements-functions-debug-info = true
        "#})
        .dep_starknet()
        .build(&t);
    Scarb::quick_snapbox()
        .arg("build")
        .arg("--test")
        .current_dir(&t)
        .assert()
        .success();

    let lib_sierra_string = t
        .child("target/dev/hello_unittest.test.sierra.json")
        .read_to_string();
    let lib_sierra = serde_json::from_str::<VersionedProgram>(&lib_sierra_string).unwrap();

    let debug_info = lib_sierra
        .into_v1()
        .unwrap()
        .debug_info
        .expect("Expected debug info to exist");
    let mappings = debug_info
        .annotations
        .get("github.com/software-mansion/cairo-profiler")
        .expect("Expected cairo-profiler annotations to exist")
        .get("statements_functions")
        .expect("Expected statements_functions info to exist");
    assert!(
        serde_json::from_value::<HashMap<StatementIdx, Vec<String>>>(mappings.clone()).is_ok(),
        "Expected statements_functions info to be a map"
    );
}

#[test]
fn add_statements_code_locations_debug_info_to_tests() {
    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello")
        .lib_cairo(indoc! {r##"
            #[starknet::interface]
            pub trait IHelloStarknet<TContractState> {
                fn increase_balance(ref self: TContractState, amount: felt252);
                fn get_balance(self: @TContractState) -> felt252;
            }

            #[starknet::contract]
            mod HelloStarknet {
                #[storage]
                struct Storage {
                    balance: felt252,
                }

                #[abi(embed_v0)]
                impl HelloStarknetImpl of super::IHelloStarknet<ContractState> {
                    fn increase_balance(ref self: ContractState, amount: felt252) {
                        assert(amount != 0, 'Amount cannot be 0');
                        self.balance.write(self.balance.read() + amount);
                    }

                    fn get_balance(self: @ContractState) -> felt252 {
                        self.balance.read()
                    }
                }
            }

            fn foo(mut shape: Span<usize>) -> usize {
                let mut result: usize = 1;

                loop {
                    match shape.pop_front() {
                        Option::Some(item) => { result *= *item; },
                        Option::None => { break; }
                    };
                };

                result
            }

            fn main() -> usize {
                foo(array![1, 2].span())
            }
        "##})
        .manifest_extra(indoc! {r#"
            [[target.starknet-contract]]

            [cairo]
            unstable-add-statements-code-locations-debug-info = true
        "#})
        .dep_starknet()
        .build(&t);
    Scarb::quick_snapbox()
        .arg("build")
        .arg("--test")
        .current_dir(&t)
        .assert()
        .success();

    let lib_sierra_string = t
        .child("target/dev/hello_unittest.test.sierra.json")
        .read_to_string();
    let lib_sierra = serde_json::from_str::<VersionedProgram>(&lib_sierra_string).unwrap();

    let debug_info = lib_sierra
        .into_v1()
        .unwrap()
        .debug_info
        .expect("Expected debug info to exist");
    let mappings = debug_info
        .annotations
        .get("github.com/software-mansion/cairo-coverage")
        .expect("Expected cairo-coverage annotations to exist")
        .get("statements_code_locations")
        .expect("Expected statements_code_locations info to exist");
    assert!(
        serde_json::from_value::<HashMap<StatementIdx, Vec<(String, SourceCodeSpan)>>>(
            mappings.clone()
        )
        .is_ok(),
        "Expected statements_code_locations info to be a map"
    );
}

#[test]
fn can_import_from_self_by_name() {
    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello")
        .lib_cairo(indoc! {r#"
            fn fib(mut n: u32) -> u32 {
                let mut a: u32 = 0;
                let mut b: u32 = 1;
                while n != 0 {
                    n = n - 1;
                    let temp = b;
                    b = a + b;
                    a = temp;
                };
                a
            }
            
            mod some {
                use hello::fib;
                
                fn main() -> u32 {
                    fib(16)
                }
            }
        "#})
        .build(&t);
    Scarb::quick_snapbox()
        .arg("build")
        .current_dir(&t)
        .assert()
        .success();
}

#[test]
fn can_build_with_add_redeposit_gas() {
    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello")
        .lib_cairo(indoc! {r#"
            fn main() -> u32 {
                fib(16)
            }

            fn fib(mut n: u32) -> u32 {
                let mut a: u32 = 0;
                let mut b: u32 = 1;
                while n != 0 {
                    n = n - 1;
                    let temp = b;
                    b = a + b;
                    a = temp;
                };
                a
            }
        "#})
        .manifest_extra(indoc! {r#"
            [cairo]
            add-redeposit-gas = true
        "#})
        .build(&t);
    Scarb::quick_snapbox()
        .arg("build")
        .current_dir(&t)
        .assert()
        .success();
}
