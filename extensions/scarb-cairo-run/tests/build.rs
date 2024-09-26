use assert_fs::TempDir;
use indoc::indoc;
use snapbox::cmd::OutputAssert;

use scarb_test_support::command::Scarb;
use scarb_test_support::project_builder::ProjectBuilder;

#[test]
fn can_run_default_main_function() {
    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello")
        .version("0.1.0")
        .lib_cairo(indoc! {r#"
            fn main() -> felt252 {
                42
            }
        "#})
        .build(&t);
    Scarb::quick_snapbox()
        .arg("cairo-run")
        .current_dir(&t)
        .assert()
        .success()
        .stdout_matches(indoc! {r#"
        [..]Compiling hello v0.1.0 ([..]Scarb.toml)
        [..]Finished `dev` profile target(s) in [..]
        [..]Running hello
        Run completed successfully, returning [42]
        "#});
}

#[test]
fn can_run_default_main_function_with_plugin() {
    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello")
        .version("0.1.0")
        .lib_cairo(indoc! {r#"
            fn main() -> felt252 {
                42
            }
        "#})
        .dep_cairo_run()
        .build(&t);
    Scarb::quick_snapbox()
        .arg("cairo-run")
        .current_dir(&t)
        .assert()
        .success()
        .stdout_matches(indoc! {r#"
        [..]Compiling hello v0.1.0 ([..]Scarb.toml)
        [..]Finished `dev` profile target(s) in [..]
        [..]Running hello
        Run completed successfully, returning [42]
        "#});
}

#[test]
fn no_entrypoint_fails() {
    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello")
        .version("0.1.0")
        .lib_cairo(indoc! {r#"
            fn hello() -> felt252 {
                42
            }
        "#})
        .dep_cairo_run()
        .build(&t);
    output_assert(
        Scarb::quick_snapbox()
            .arg("cairo-run")
            .current_dir(&t)
            .assert()
            .failure(),
        indoc! {r#"
        [..]Compiling hello v0.1.0 ([..]Scarb.toml)
        [..]Finished `dev` profile target(s) in [..]
        [..]Running hello
        error: Function with suffix `::main` to run not found.
        "#},
    )
}

#[test]
fn no_debug_build_fails() {
    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello")
        .version("0.1.0")
        .manifest_extra(indoc! {r#"
            [cairo]
            sierra-replace-ids = false
        "#})
        .lib_cairo(indoc! {r#"
            fn main() -> felt252 {
                42
            }
        "#})
        .dep_cairo_run()
        .build(&t);
    output_assert(
        Scarb::quick_snapbox()
            .arg("cairo-run")
            .current_dir(&t)
            .assert()
            .failure(),
        indoc! {r#"
        [..]Compiling hello v0.1.0 ([..]Scarb.toml)
        [..]Finished `dev` profile target(s) in [..]
        [..]Running hello
        error: Function with suffix `::main` to run not found.
        "#},
    )
}

#[test]
fn can_run_executable() {
    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello")
        .version("0.1.0")
        // Note we can use executables even without debug names!
        .manifest_extra(indoc! {r#"
            [cairo]
            sierra-replace-ids = false
        "#})
        .lib_cairo(indoc! {r#"
            #[main]
            fn hello() -> felt252 {
                42
            }
        "#})
        .dep_cairo_run()
        .build(&t);
    Scarb::quick_snapbox()
        .arg("cairo-run")
        .current_dir(&t)
        .assert()
        .success()
        .stdout_matches(indoc! {r#"
        [..]Compiling hello v0.1.0 ([..]Scarb.toml)
        [..]Finished `dev` profile target(s) in [..]
        [..]Running hello
        Run completed successfully, returning [42]
        "#});
}

#[test]
fn ambiguous_executables_will_fail() {
    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello")
        .version("0.1.0")
        .lib_cairo(indoc! {r#"
            #[main]
            fn hello() -> felt252 {
                42
            }
            #[main]
            fn world() -> felt252 {
                53
            }
        "#})
        .dep_cairo_run()
        .build(&t);
    output_assert(
        Scarb::quick_snapbox()
            .arg("cairo-run")
            .current_dir(&t)
            .assert()
            .failure(),
        indoc! {r#"
        [..]Compiling hello v0.1.0 ([..]Scarb.toml)
        [..]Finished `dev` profile target(s) in [..]
        [..]Running hello
        error: multiple executable functions found
        please choose a function to run from the list:
        `hello::hello`, `hello::world`
        "#},
    )
}

#[test]
fn ambiguous_executables_will_fail_no_debug_names() {
    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello")
        .version("0.1.0")
        .manifest_extra(indoc! {r#"
            [cairo]
            sierra-replace-ids = false
        "#})
        .lib_cairo(indoc! {r#"
            #[main]
            fn hello() -> felt252 {
                42
            }
            #[main]
            fn world() -> felt252 {
                53
            }
        "#})
        .dep_cairo_run()
        .build(&t);
    output_assert(
        Scarb::quick_snapbox()
            .arg("cairo-run")
            .current_dir(&t)
            .assert()
            .failure(),
        // Note that we cannot list available executables, as we don't know their debug names.
        indoc! {r#"
        [..]Compiling hello v0.1.0 ([..]Scarb.toml)
        [..]Finished `dev` profile target(s) in [..]
        [..]Running hello
        error: multiple executable functions found
        please only mark a single function as executable or enable debug ids and choose function by name
        "#},
    )
}

#[test]
fn can_choose_function_to_run_by_name() {
    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello")
        .version("0.1.0")
        .lib_cairo(indoc! {r#"
            fn a() {
                println!("A");
            }
            fn main() {
                println!("M");
            }
            fn b() {
                println!("B");
            }
        "#})
        .dep_cairo_run()
        .build(&t);
    Scarb::quick_snapbox()
        .arg("--quiet")
        .arg("cairo-run")
        .arg("--function")
        .arg("b")
        .current_dir(&t)
        .assert()
        .success()
        .stdout_matches(indoc! {r#"
        B
        "#});
    Scarb::quick_snapbox()
        .arg("--quiet")
        .arg("cairo-run")
        .current_dir(&t)
        .assert()
        .success()
        .stdout_matches(indoc! {r#"
        M
        "#});
}

#[test]
fn cannot_choose_non_executable_if_any_present() {
    // This is not possible as of current Cairo compiler implementation.
    // If executables are found, only functions marked as executable are compiled.
    // All other entrypoints are removed from the generated Sierra code.
    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello")
        .version("0.1.0")
        .lib_cairo(indoc! {r#"
            fn a() {
                println!("A");
            }
            fn main() {
                println!("M");
            }
            #[main]
            fn b() {
                println!("B");
            }
        "#})
        .dep_cairo_run()
        .build(&t);
    output_assert(
        Scarb::quick_snapbox()
            .arg("cairo-run")
            .arg("--function")
            .arg("a")
            .current_dir(&t)
            .assert()
            .failure(),
        indoc! {r#"
            [..]Compiling hello v0.1.0 ([..]Scarb.toml)
            [..]Finished `dev` profile target(s) in [..]
            [..]Running hello
            error: Function with suffix `::a` to run not found.
        "#},
    )
}

#[test]
fn can_choose_executable_to_run() {
    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello")
        .version("0.1.0")
        .lib_cairo(indoc! {r#"
            fn hello() -> felt252 {
                64
            }
            #[main]
            fn world() -> felt252 {
                53
            }
            mod something {
                #[main]
                fn hello() -> felt252 {
                    42
                }
            }
        "#})
        .dep_cairo_run()
        .build(&t);
    Scarb::quick_snapbox()
        .arg("cairo-run")
        .arg("--function")
        .arg("hello")
        .current_dir(&t)
        .assert()
        .success()
        .stdout_matches(indoc! {r#"
            [..]Compiling hello v0.1.0 ([..]Scarb.toml)
            [..]Finished `dev` profile target(s) in [..]
            [..]Running hello
            Run completed successfully, returning [42]
        "#});
}

#[test]
fn choose_not_existing_function() {
    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello")
        .version("0.1.0")
        .lib_cairo(indoc! {r#"
            fn main() {
                println!("main");
            }
        "#})
        .build(&t);
    output_assert(
        Scarb::quick_snapbox()
            .arg("cairo-run")
            .arg("--function")
            .arg("b")
            .current_dir(&t)
            .assert()
            .failure(),
        indoc! {r#"
        [..]Compiling hello v0.1.0 ([..]Scarb.toml)
        [..]Finished `dev` profile target(s) in [..]
        [..]Running hello
        [..]error: Function with suffix `::b` to run not found.
    "#},
    )
}

fn output_assert(output: OutputAssert, expected: &str) {
    #[cfg(windows)]
    output.stdout_matches(format!(
        "{expected}error: process did not exit successfully: exit code: 1\n"
    ));
    #[cfg(not(windows))]
    output.stdout_matches(expected);
}
