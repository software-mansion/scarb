use assert_fs::TempDir;
use assert_fs::fixture::PathChild;
use assert_fs::prelude::PathAssert;
use indoc::{formatdoc, indoc};
use predicates::prelude::predicate;
use scarb_test_support::command::Scarb;
use scarb_test_support::fixtures::{build_executable_project, executable_project_builder};
use scarb_test_support::fsx::ChildPathEx;
use scarb_test_support::project_builder::ProjectBuilder;
use snapbox::{Assert, Data, Redactions};
use test_case::test_case;

#[test_case("standalone")]
#[test_case("bootloader")]
fn can_execute_default_main_function_from_executable(target: &str) {
    let t = build_executable_project();
    Scarb::quick_command()
        .arg("execute")
        .arg(format!("--target={target}"))
        .arg("--output=standard")
        .current_dir(&t)
        .assert()
        .success()
        .stdout_eq(indoc! {r#"
        [..]Compiling hello v0.1.0 ([..]Scarb.toml)
        [..]Finished `dev` profile target(s) in [..]
        [..]Executing hello
        Saving output to: target/execute/hello/execution1
        "#});

    t.child("target/execute/hello/execution1/prover_input.json")
        .assert_is_json::<serde_json::Value>();
}

#[test_case("standalone")]
#[test_case("bootloader")]
fn can_execute_prebuilt_executable(target: &str) {
    let t = build_executable_project();
    Scarb::quick_command().arg("build").current_dir(&t).assert();
    Scarb::quick_command()
        .arg("execute")
        .arg(format!("--target={target}"))
        .arg("--output=standard")
        .arg("--no-build")
        .current_dir(&t)
        .assert()
        .success()
        .stdout_eq(indoc! {r#"
        [..]Executing hello
        Saving output to: target/execute/hello/execution1
        "#});

    t.child("target/execute/hello/execution1/prover_input.json")
        .assert_is_json::<serde_json::Value>();
}

#[test]
fn cannot_produce_cairo_pie_for_standalone_target() {
    let t = build_executable_project();
    Scarb::quick_command()
        .arg("execute")
        .arg("--target=standalone")
        .arg("--output=cairo-pie")
        .current_dir(&t)
        .assert()
        .failure()
        .stdout_eq(indoc! {r#"
            error: Cairo pie output format is not supported for standalone execution target
        "#});
}

#[test]
fn can_produce_cairo_pie_for_bootloader_target() {
    let t = build_executable_project();
    Scarb::quick_command()
        .arg("execute")
        .arg("--target=bootloader")
        .arg("--output=cairo-pie")
        .current_dir(&t)
        .assert()
        .success()
        .stdout_eq(indoc! {r#"
        [..]Compiling hello v0.1.0 ([..]Scarb.toml)
        [..]Finished `dev` profile target(s) in [..]
        [..]Executing hello
        [..]Saving output to: target/execute/hello/execution1/cairo_pie.zip
        "#});

    t.child("target/execute/hello/execution1/cairo_pie.zip")
        .assert(predicate::path::exists());
}

#[test]
fn fails_when_attr_missing() {
    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello")
        .version("0.1.0")
        .dep_cairo_execute()
        .manifest_extra(indoc! {r#"
            [executable]

            [cairo]
            enable-gas = false
        "#})
        .lib_cairo(indoc! {r#"
            fn main() -> felt252 {
                42
            }
        "#})
        .build(&t);

    Scarb::quick_command()
        .arg("execute")
        .current_dir(&t)
        .assert()
        .failure()
        .stdout_eq(indoc! {r#"
            [..]Compiling hello v0.1.0 ([..]Scarb.toml)
            error: requested `#[executable]` not found
            error: could not compile `hello` due to [..] previous error
            error: `scarb` command exited with error
        "#});

    Scarb::quick_command()
        .arg("execute")
        .arg("--no-build")
        .current_dir(&t)
        .assert()
        .with_assert(Assert::default().redact_with(Redactions::default()))
        .failure()
        .stdout_eq(indoc! {r#"
            [..]Executing hello
            error: package has not been compiled, file does not exist: `hello.executable.json`
            help: run `scarb build` to compile the package

        "#});
}

#[test_case("standalone", "error: Panicked with \"abcd\".")]
#[test_case("bootloader", "error: Panicked with \"abcd\".")]
fn can_print_panic_reason(target: &str, panic: &str) {
    let t = TempDir::new().unwrap();
    executable_project_builder()
        .lib_cairo(indoc! {r#"
            #[executable]
            fn main() -> felt252 {
                if true {
                    panic!("abcd");
                }
                42
            }
        "#})
        .build(&t);

    Scarb::quick_command()
        .arg("execute")
        .arg(format!("--target={target}"))
        .current_dir(&t)
        .assert()
        .failure()
        .stdout_eq(formatdoc! {r#"
            [..]Compiling hello v0.1.0 ([..]Scarb.toml)
            [..]Finished `dev` profile target(s) in [..]
            [..]Executing hello
            {panic}
        "#});
}

#[test]
fn no_target_defined() {
    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello_world")
        .dep_cairo_test()
        .dep_starknet()
        .dep_cairo_execute()
        .manifest_extra(indoc! {r#"
            [cairo]
            enable-gas = false
        "#})
        .lib_cairo(indoc! {r#"
            #[executable]
            fn main() -> felt252 {
                42
            }

            #[executable]
            fn secondary() -> felt252 {
                42
            }
        "#})
        .build(&t);

    Scarb::quick_command()
        .arg("execute")
        .arg("--no-build")
        .current_dir(&t)
        .assert()
        .failure()
        .stdout_eq(indoc! {r#"
            error: no executable target found for package `hello_world`
            help: you can add `executable` target to the package manifest with following excerpt
            -> Scarb.toml
                [executable]

                [dependencies]
                cairo_execute = "[..].[..].[..]"

        "#});
}

#[test]
fn undefined_target_specified() {
    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello_world")
        .dep_cairo_test()
        .dep_starknet()
        .dep_cairo_execute()
        .manifest_extra(indoc! {r#"
            [executable]

            [cairo]
            enable-gas = false
        "#})
        .lib_cairo(indoc! {r#"
            #[executable]
            fn main() -> felt252 {
                42
            }

            #[executable]
            fn secondary() -> felt252 {
                42
            }
        "#})
        .build(&t);

    Scarb::quick_command()
        .arg("execute")
        .arg("--executable-name=secondary")
        .arg("--no-build")
        .current_dir(&t)
        .assert()
        .failure()
        .stdout_eq(indoc! {r#"
            error: no executable target with name `secondary` found for package `hello_world`
        "#});

    Scarb::quick_command()
        .arg("execute")
        .arg("--executable-function=secondary")
        .arg("--no-build")
        .current_dir(&t)
        .assert()
        .failure()
        .stdout_eq(indoc! {r#"
            error: no executable target with executable function `secondary` found for package `hello_world`
        "#});
}

fn two_targets() -> ProjectBuilder {
    ProjectBuilder::start()
        .name("hello_world")
        .dep_cairo_test()
        .dep_starknet()
        .dep_cairo_execute()
        .manifest_extra(indoc! {r#"
            [executable]
            function = "hello_world::main"

            [[target.executable]]
            name = "secondary"
            function = "hello_world::secondary"

            [cairo]
            enable-gas = false
        "#})
        .lib_cairo(indoc! {r#"
            #[executable]
            fn main() -> felt252 {
                24
            }

            #[executable]
            fn secondary() -> felt252 {
                42
            }
        "#})
}

#[test]
fn can_choose_build_target() {
    let t = TempDir::new().unwrap();
    two_targets().build(&t);

    Scarb::quick_command()
        .arg("execute")
        .arg("--executable-name=secondary")
        .arg("--print-program-output")
        .current_dir(&t)
        .assert()
        .success()
        .stdout_eq(indoc! {r#"
            [..]Compiling executable(hello_world) hello_world v1.0.0 ([..]Scarb.toml)
            [..]Compiling executable(secondary) hello_world v1.0.0 ([..]Scarb.toml)
            [..]Finished `dev` profile target(s) in [..]
            [..]Executing hello_world
            Program output:
            42
        "#});

    // Re-using the same build artifact
    Scarb::quick_command()
        .arg("execute")
        .arg("--no-build")
        .arg("--executable-function=hello_world::main")
        .arg("--print-program-output")
        .current_dir(&t)
        .assert()
        .success()
        .stdout_eq(indoc! {r#"
            [..]Executing hello_world
            Program output:
            24
        "#});
}

#[test]
fn executable_must_be_chosen() {
    let t = TempDir::new().unwrap();
    two_targets().build(&t);

    Scarb::quick_command()
        .arg("execute")
        .arg("--no-build")
        .current_dir(&t)
        .assert()
        .failure()
        .stdout_eq(indoc! {r#"
            error: more than one executable target found for package `hello_world`
            help: specify the target with `--executable-name` or `--executable-function`

        "#});
}

#[test]
fn can_set_ui_verbosity() {
    let t = build_executable_project();
    Scarb::quick_command()
        .arg("execute")
        .arg("--quiet")
        .current_dir(&t)
        .assert()
        .success()
        .stdout_eq(Data::from("").raw());
}

#[test]
fn maintains_parent_verbosity() {
    let t = build_executable_project();
    Scarb::quick_command()
        .arg("--quiet")
        .arg("execute")
        .current_dir(&t)
        .assert()
        .success()
        .stdout_eq(Data::from("").raw());
}

#[test]
fn can_use_features() {
    let t = TempDir::new().unwrap();
    executable_project_builder()
        .manifest_extra(indoc! {r#"
            [executable]
            [cairo]
            enable-gas = false
            [features]
            x = []
        "#})
        .lib_cairo(indoc! {r#"
            #[cfg(feature: 'x')]
            fn f() -> felt252 { 21 }

            #[executable]
            fn main() -> felt252 { f() }
        "#})
        .build(&t);

    Scarb::quick_command()
        .arg("execute")
        .arg("--features=x")
        .current_dir(&t)
        .assert()
        .success()
        .stdout_eq(indoc! {r#"
        [..]Compiling hello v0.1.0 ([..]Scarb.toml)
        [..]Finished `dev` profile target(s) in [..]
        [..]Executing hello
        "#});
}

#[test_case("standalone")]
#[test_case("bootloader")]
fn can_create_profiler_trace_file(target: &str) {
    let t = TempDir::new().unwrap();
    executable_project_builder()
        .manifest_extra(indoc! {r#"
            [executable]
            sierra = true
            [cairo]
            enable-gas = false
        "#})
        .lib_cairo(indoc! {r#"
            #[executable]
            fn main() -> felt252 { 1 }
        "#})
        .build(&t);

    Scarb::quick_command()
        .arg("execute")
        .arg(format!("--target={target}"))
        .arg("--output=standard")
        .arg("--save-profiler-trace-data")
        .current_dir(&t)
        .assert()
        .success()
        .stdout_eq(indoc! {r#"
        [..]Compiling hello v0.1.0 ([..]Scarb.toml)
        [..]Finished `dev` profile target(s) in [..]
        [..]Executing hello
        Saving output to: target/execute/hello/execution1
        Profiler tracked resource: cairo-steps
        Saving profiler trace data to: [..]
        "#});

    t.child("target/execute/hello/execution1/cairo_profiler_trace.json")
        .assert_is_json::<serde_json::Value>();
}

#[test]
fn no_required_sierra_for_profiler_trace_file() {
    let t = TempDir::new().unwrap();
    executable_project_builder()
        .manifest_extra(indoc! {r#"
            [executable]
            [cairo]
            enable-gas = false
        "#})
        .lib_cairo(indoc! {r#"
            #[executable]
            fn main() -> felt252 { 1 }
        "#})
        .build(&t);

    Scarb::quick_command()
        .arg("execute")
        .arg("--save-profiler-trace-data")
        .current_dir(&t)
        .assert()
        .failure()
        .stdout_eq(indoc! {r#"
        [..]Compiling hello v0.1.0 ([..]Scarb.toml)
        [..]Finished `dev` profile target(s) in [..]
        [..]Executing hello
        error: Failed to write profiler trace data into a file — missing sierra code for target `hello`. Set `sierra = true` under your `[executable]` target in the config and try again.
        "#});
}

#[test]
fn no_build_artifact_for_profiler_trace_file() {
    let t = TempDir::new().unwrap();
    executable_project_builder()
        .manifest_extra(indoc! {r#"
            [executable]
            sierra = true
            [cairo]
            enable-gas = false
        "#})
        .lib_cairo(indoc! {r#"
            #[executable]
            fn main() -> felt252 { 1 }
        "#})
        .build(&t);

    Scarb::quick_command().arg("build").current_dir(&t).assert();
    let artifact_path = t.path().join("target/dev/hello.executable.sierra.json");
    if artifact_path.exists() {
        std::fs::remove_file(&artifact_path).unwrap();
    }
    let assert = Scarb::quick_command()
        .arg("execute")
        .arg("--no-build")
        .arg("--save-profiler-trace-data")
        .current_dir(&t)
        .assert();
    let assert = assert.with_assert(Assert::default().redact_with(Redactions::default()));
    assert
        .failure()
        .stdout_eq(indoc! {r#"
        [..]Executing hello
        error: Missing sierra code for executable `hello`, file [..]hello.executable.sierra.json does not exist. help: run `scarb build` to compile the package and try again.
        "#});
}

#[test]
fn invalid_tracked_resource_for_profiler_trace_file() {
    let t = TempDir::new().unwrap();
    executable_project_builder()
        .manifest_extra(indoc! {r#"
            [executable]
            sierra = true
            [cairo]
            enable-gas = false
            [tool.cairo-profiler]
            tracked-resource = "whatever"
        "#})
        .lib_cairo(indoc! {r#"
            #[executable]
            fn main() -> felt252 { 1 }
        "#})
        .build(&t);

    Scarb::quick_command()
        .arg("execute")
        .arg("--save-profiler-trace-data")
        .current_dir(&t)
        .assert()
        .failure()
        .stdout_eq(indoc! {r#"
        [..]Compiling hello v0.1.0 ([..]Scarb.toml)
        [..]Finished `dev` profile target(s) in [..]
        [..]Executing hello
        error: Invalid tracked resource set for profiler: whatever
        help: valid options are: `cairo-steps` or `sierra-gas`
        "#});
}

#[test]
fn allow_syscalls_triggers_layout_warning() {
    let t = TempDir::new().unwrap();
    executable_project_builder()
        .manifest_extra(indoc! {r#"
            [executable]
            allow-syscalls = true

            [cairo]
            enable-gas = false
        "#})
        .build(&t);
    Scarb::quick_command()
        .arg("execute")
        .current_dir(&t)
        .assert()
        .success()
        .stdout_eq(indoc! {r#"
        [..]Compiling hello v0.1.0 ([..]Scarb.toml)
        [..]Finished `dev` profile target(s) in [..]
        warn: the executable target hello you are trying to execute has `allow-syscalls` set to `true`
        if your executable uses syscalls, it cannot be run with `all_cairo_stwo` layout
        please use `--layout` flag to specify a different layout, for example: `--layout=all_cairo`

        [..]Executing hello
        "#});
}
