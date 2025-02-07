use assert_fs::assert::PathAssert;
use assert_fs::fixture::PathChild;
use assert_fs::TempDir;
use indoc::indoc;
use predicates::prelude::*;
use scarb_test_support::command::Scarb;
use scarb_test_support::fsx::ChildPathEx;
use scarb_test_support::predicates::is_file_empty;
use scarb_test_support::project_builder::ProjectBuilder;
use snapbox::cmd::OutputAssert;

fn executable_project_builder() -> ProjectBuilder {
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
            #[executable]
            fn main() -> felt252 {
                42
            }
        "#})
}

fn build_executable_project() -> TempDir {
    let t = TempDir::new().unwrap();
    executable_project_builder().build(&t);
    t
}

#[test]
fn can_execute_default_main_function_from_executable() {
    let t = build_executable_project();
    Scarb::quick_snapbox()
        .arg("execute")
        .current_dir(&t)
        .assert()
        .success()
        .stdout_matches(indoc! {r#"
        [..]Compiling hello v0.1.0 ([..]Scarb.toml)
        [..]Finished `dev` profile target(s) in [..]
        [..]Executing hello
        Saving output to: target/execute/hello/execution1
        "#});

    t.child("target/execute/hello/execution1/air_private_input.json")
        .assert_is_json::<serde_json::Value>();
    t.child("target/execute/hello/execution1/air_public_input.json")
        .assert_is_json::<serde_json::Value>();
    t.child("target/execute/hello/execution1/memory.bin")
        .assert(predicates::path::exists().and(is_file_empty().not()));
    t.child("target/execute/hello/execution1/trace.bin")
        .assert(predicates::path::exists().and(is_file_empty().not()));
}

#[test]
fn can_execute_prebuilt_executable() {
    let t = build_executable_project();
    Scarb::quick_snapbox().arg("build").current_dir(&t).assert();
    Scarb::quick_snapbox()
        .arg("execute")
        .arg("--no-build")
        .current_dir(&t)
        .assert()
        .success()
        .stdout_matches(indoc! {r#"
        [..]Executing hello
        Saving output to: target/execute/hello/execution1
        "#});

    t.child("target/execute/hello/execution1/air_private_input.json")
        .assert_is_json::<serde_json::Value>();
    t.child("target/execute/hello/execution1/air_public_input.json")
        .assert_is_json::<serde_json::Value>();
    t.child("target/execute/hello/execution1/memory.bin")
        .assert(predicates::path::exists().and(is_file_empty().not()));
    t.child("target/execute/hello/execution1/trace.bin")
        .assert(predicates::path::exists().and(is_file_empty().not()));
}

#[test]
fn can_execute_bootloader_target() {
    let t = build_executable_project();
    Scarb::quick_snapbox()
        .arg("execute")
        .arg("--target=bootloader")
        .current_dir(&t)
        .assert()
        .success()
        .stdout_matches(indoc! {r#"
        [..]Compiling hello v0.1.0 ([..]Scarb.toml)
        [..]Finished `dev` profile target(s) in [..]
        [..]Executing hello
        Saving output to: target/execute/hello/execution1/cairo_pie.zip
        "#});

    t.child("target/execute/hello/execution1/cairo_pie.zip")
        .assert(predicates::path::exists());
}

#[test]
fn cannot_produce_trace_file_for_bootloader_target() {
    let t = build_executable_project();
    let output = Scarb::quick_snapbox()
        .arg("execute")
        .arg("--target=bootloader")
        .arg("--output=standard")
        .current_dir(&t)
        .assert()
        .failure();
    output_assert(
        output,
        indoc! {r#"
        error: Standard output format is not supported for bootloader execution target
        "#},
    );
}

#[test]
fn cannot_produce_cairo_pie_for_standalone_target() {
    let t = build_executable_project();
    let output = Scarb::quick_snapbox()
        .arg("execute")
        .arg("--target=standalone")
        .arg("--output=cairo-pie")
        .current_dir(&t)
        .assert()
        .failure();
    output_assert(
        output,
        indoc! {r#"
        error: Cairo pie output format is not supported for standalone execution target
        "#},
    );
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

    output_assert(
        Scarb::quick_snapbox()
            .arg("execute")
            .current_dir(&t)
            .assert()
            .failure(),
        indoc! {r#"
        [..]Compiling hello v0.1.0 ([..]Scarb.toml)
        error: Requested `#[executable]` not found.
        error: could not compile `hello` due to previous error
        error: `scarb metadata` exited with error
        "#},
    );

    output_assert(
        Scarb::quick_snapbox()
            .arg("execute")
            .arg("--no-build")
            .current_dir(&t)
            .assert()
            .failure(),
        indoc! {r#"
        [..]Executing hello
        error: package has not been compiled, file does not exist: `hello.executable.json`
        help: run `scarb build` to compile the package

        "#},
    );
}

#[test]
fn can_print_panic_reason() {
    let t = TempDir::new().unwrap();
    executable_project_builder()
        .lib_cairo(indoc! {r#"
            #[executable]
            fn main() -> felt252 {
                panic!("abcd");
                42
            }
        "#})
        .build(&t);
    let output = Scarb::quick_snapbox()
        .arg("execute")
        .arg("--print-program-output")
        .arg("--print-resource-usage")
        .current_dir(&t)
        .assert()
        .failure();

    output_assert(
        output,
        indoc! {r#"
        [..]Compiling hello v0.1.0 ([..]Scarb.toml)
        [..]Finished `dev` profile target(s) in [..]
        [..]Executing hello
        Program output:
        1
        Resources:
        	steps: [..]
        	memory holes: [..]
        	builtins: ([..])
        	syscalls: ()
        Saving output to: target/execute/hello/execution1
        error: Panicked with "abcd".
        "#},
    );
    t.child("target/execute/hello/execution1/air_private_input.json")
        .assert_is_json::<serde_json::Value>();
    t.child("target/execute/hello/execution1/air_public_input.json")
        .assert_is_json::<serde_json::Value>();
    t.child("target/execute/hello/execution1/memory.bin")
        .assert(predicates::path::exists().and(is_file_empty().not()));
    t.child("target/execute/hello/execution1/trace.bin")
        .assert(predicates::path::exists().and(is_file_empty().not()));
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

    let output = Scarb::quick_snapbox()
        .arg("execute")
        .arg("--no-build")
        .current_dir(&t)
        .assert()
        .failure();
    output_assert(
        output,
        indoc! {r#"
        error: no executable target found for package `hello_world`
        help: you can add `executable` target to the package manifest with following excerpt
        -> Scarb.toml
            [executable]

            [dependencies]
            cairo_execute = "[..].[..].[..]"

    "#},
    );
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

    let output = Scarb::quick_snapbox()
        .arg("execute")
        .arg("--executable-name=secondary")
        .arg("--no-build")
        .current_dir(&t)
        .assert()
        .failure();
    output_assert(
        output,
        "error: no executable target with name `secondary` found for package `hello_world`\n",
    );

    let output = Scarb::quick_snapbox()
        .arg("execute")
        .arg("--executable-function=secondary")
        .arg("--no-build")
        .current_dir(&t)
        .assert()
        .failure();
    output_assert(
        output,
        "error: no executable target with executable function `secondary` found for package `hello_world`\n",
    );
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

    Scarb::quick_snapbox()
        .arg("execute")
        .arg("--executable-name=secondary")
        .arg("--print-program-output")
        .current_dir(&t)
        .assert()
        .success()
        .stdout_matches(indoc! {r#"
            [..]Compiling executable(hello_world) hello_world v1.0.0 ([..]Scarb.toml)
            [..]Compiling executable(secondary) hello_world v1.0.0 ([..]Scarb.toml)
            [..]Finished `dev` profile target(s) in [..]
            [..]Executing hello_world
            Program output:
            0
            42
            Saving output to: target/execute/hello_world/execution1
        "#});

    // Re-using the same build artifact
    Scarb::quick_snapbox()
        .arg("execute")
        .arg("--no-build")
        .arg("--executable-function=hello_world::main")
        .arg("--print-program-output")
        .current_dir(&t)
        .assert()
        .success()
        .stdout_matches(indoc! {r#"
            [..]Executing hello_world
            Program output:
            0
            24
            Saving output to: target/execute/hello_world/execution2
        "#});
}

#[test]
fn executable_must_be_chosen() {
    let t = TempDir::new().unwrap();
    two_targets().build(&t);

    let output = Scarb::quick_snapbox()
        .arg("execute")
        .arg("--no-build")
        .current_dir(&t)
        .assert()
        .failure();

    output_assert(
        output,
        indoc! {r#"
            error: more than one executable target found for package `hello_world`
            help: specify the target with `--executable-name` or `--executable-function`
            
        "#},
    );
}

fn output_assert(output: OutputAssert, expected: &str) {
    #[cfg(windows)]
    output.stdout_matches(format!(
        "{expected}error: process did not exit successfully: exit code: 1\n"
    ));
    #[cfg(not(windows))]
    output.stdout_matches(expected);
}
