use assert_fs::assert::PathAssert;
use assert_fs::fixture::PathChild;
use assert_fs::TempDir;
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

    Scarb::quick_snapbox()
        .env("SCARB_CACHE", cache_dir.path())
        .arg("check")
        .current_dir(&t)
        .assert()
        .stdout_matches(indoc! { r#"
        [..]Checking hello v0.1.0 ([..]Scarb.toml)
        [..]Finished checking release target(s) in [..]
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
            error: Skipped tokens. Expected: Const/Module/Use/FreeFunction/ExternFunction/ExternType/Trait/Impl/Struct/Enum/TypeAlias/InlineMacro or an attribute.
             --> [..]/lib.cairo:1:1
            not_a_keyword
            ^***********^


            error: could not check `hello` due to previous error
        "#});
}
