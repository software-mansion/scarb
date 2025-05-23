use assert_fs::TempDir;
use assert_fs::fixture::{ChildPath, FileWriteStr, PathCreateDir};
use assert_fs::prelude::PathChild;
use indoc::indoc;
use libloading::library_filename;
use scarb_test_support::cairo_plugin_project_builder::CairoPluginProjectBuilder;
use scarb_test_support::command::Scarb;
use scarb_test_support::project_builder::ProjectBuilder;
use scarb_test_support::workspace_builder::WorkspaceBuilder;
use snapbox::cmd::Command;
use std::fs;
use std::sync::LazyLock;

static TRIPLETS: [(&str, &str); 4] = [
    ("aarch64-apple-darwin", ".dylib"),
    ("x86_64-apple-darwin", ".dylib"),
    ("x86_64-unknown-linux-gnu", ".so"),
    ("x86_64-pc-windows-msvc", ".dll"),
];

static EXAMPLE_NAME: &str = "proc_macro_example";
static EXAMPLE_VERSION: &str = "0.1.0";

fn build_example_project(t: &impl PathChild) {
    CairoPluginProjectBuilder::default()
        .name(EXAMPLE_NAME)
        .version(EXAMPLE_VERSION)
        .lib_rs(indoc! {r#"
            use cairo_lang_macro::{ProcMacroResult, TokenStream, inline_macro};
            #[inline_macro]
            pub fn some(token_stream: TokenStream) -> ProcMacroResult {
                ProcMacroResult::new(token_stream)
            }
        "#})
        .build(t);
}

static BUILT_EXAMPLE_PROJECT: LazyLock<TempDir> = LazyLock::new(|| {
    let t = TempDir::new().unwrap();
    build_example_project(&t);
    let build_dir = t.child("cargo_build_dir");
    Command::new("cargo")
        .arg("build")
        .arg("--release")
        .env("CARGO_TARGET_DIR", build_dir.path())
        .current_dir(&t)
        .assert()
        .success();
    t
});

fn proc_macro_example(t: &ChildPath) {
    build_example_project(t);
    let dll_filename = library_filename(EXAMPLE_NAME);
    let dll_filename = dll_filename.to_string_lossy().to_string();
    let build_dir = BUILT_EXAMPLE_PROJECT.child("cargo_build_dir");
    t.child("target/scarb/cairo-plugin")
        .create_dir_all()
        .unwrap();
    for (target, extension) in TRIPLETS {
        let target_name = format!("{EXAMPLE_NAME}_v{EXAMPLE_VERSION}_{target}{extension}");
        fs::copy(
            build_dir.child("release").child(dll_filename.clone()),
            t.child("target/scarb/cairo-plugin/").child(target_name),
        )
        .unwrap();
    }
}

#[test]
fn compile_with_prebuilt_plugins() {
    let t = TempDir::new().unwrap();
    proc_macro_example(&t.child("dep"));
    let builder = |name: &str| {
        ProjectBuilder::start()
            .name(name)
            .lib_cairo(indoc! {r#"
                fn main() -> u32 {
                    let x = some!(42);
                    x
                }
            "#})
            .dep("proc_macro_example", t.child("dep"))
            .manifest_extra(indoc! {r#"
                [tool.scarb]
                allow-prebuilt-plugins = ["proc_macro_example"]
            "#})
    };
    builder("a").build(&t.child("a"));
    builder("b").build(&t.child("b"));
    WorkspaceBuilder::start()
        .add_member("a")
        .add_member("b")
        .build(&t);
    Scarb::quick_snapbox()
        .arg("build")
        // Disable Cargo and Rust compiler.
        .env("CARGO", "/bin/false")
        .env("RUSTC", "/bin/false")
        .current_dir(&t)
        .assert()
        .success()
        .stdout_matches(indoc! {r#"
            [..]Compiling a v1.0.0 ([..]Scarb.toml)
            [..]Compiling b v1.0.0 ([..]Scarb.toml)
            [..] Finished `dev` profile target(s) in [..]
        "#});
}

#[test]
fn compile_with_prebuilt_plugins_only_one_allows() {
    let t = TempDir::new().unwrap();
    proc_macro_example(&t.child("dep"));
    let builder = |name: &str, allow: bool| {
        let b = ProjectBuilder::start()
            .name(name)
            .lib_cairo(indoc! {r#"
                fn main() -> u32 {
                    let x = some!(42);
                    x
                }
            "#})
            .dep("proc_macro_example", t.child("dep"));
        if allow {
            b.manifest_extra(indoc! {r#"
                [tool.scarb]
                allow-prebuilt-plugins = ["proc_macro_example"]
            "#})
        } else {
            b
        }
    };
    builder("a", true).build(&t.child("a"));
    builder("b", false).build(&t.child("b"));
    WorkspaceBuilder::start()
        .add_member("a")
        .add_member("b")
        .build(&t);
    Scarb::quick_snapbox()
        .arg("build")
        .current_dir(&t)
        // Disable output from Cargo.
        .env("CARGO_TERM_QUIET", "true")
        .assert()
        .success()
        .stdout_matches(indoc! {r#"
            [..]Compiling proc_macro_example v0.1.0 ([..])
            [..]Compiling a v1.0.0 ([..]Scarb.toml)
            [..]Compiling b v1.0.0 ([..]Scarb.toml)
            [..] Finished `dev` profile target(s) in [..]
        "#});
}

#[test]
fn compile_valid_prebuilt_disallowed_by_flag() {
    let t = TempDir::new().unwrap();
    proc_macro_example(&t.child("dep"));
    let builder = |name: &str| {
        ProjectBuilder::start()
            .name(name)
            .lib_cairo(indoc! {r#"
                fn main() -> u32 {
                    let x = some!(42);
                    x
                }
            "#})
            .dep("proc_macro_example", t.child("dep"))
            .manifest_extra(indoc! {r#"
                [tool.scarb]
                allow-prebuilt-plugins = ["proc_macro_example"]
            "#})
    };
    builder("a").build(&t.child("a"));
    WorkspaceBuilder::start().add_member("a").build(&t);
    Scarb::quick_snapbox()
        .arg("--no-prebuilt-proc-macros")
        .arg("build")
        // Disable output from Cargo.
        .env("CARGO_TERM_QUIET", "true")
        .current_dir(&t)
        .assert()
        .success()
        .stdout_matches(indoc! {r#"
            [..]Compiling proc_macro_example v0.1.0 ([..])
            [..]Compiling a v1.0.0 ([..]Scarb.toml)
            [..] Finished `dev` profile target(s) in [..]
        "#});
}

fn invalid_prebuilt_project(t: &ChildPath) {
    let name = "invalid_prebuilt_example";
    let version = "0.1.0";
    CairoPluginProjectBuilder::default()
        .name(name)
        .version(version)
        .lib_rs(indoc! {r#"
             use cairo_lang_macro::{ProcMacroResult, TokenStream, inline_macro};
            #[inline_macro]
            pub fn some(token_stream: TokenStream) -> ProcMacroResult {
                ProcMacroResult::new(token_stream)
            }
        "#})
        .build(t);
    let target = t.child("target/scarb/cairo-plugin");
    for (triplet, extension) in TRIPLETS {
        let path = format!("{name}_v{version}_{triplet}{extension}");
        target
            .child(path)
            .write_str("this is not a valid lib")
            .unwrap();
    }
}

#[test]
fn compile_with_invalid_prebuilt_plugins() {
    let t = TempDir::new().unwrap();
    invalid_prebuilt_project(&t.child("dep"));
    ProjectBuilder::start()
        .name("hello")
        .lib_cairo(indoc! {r#"
            fn main() -> u32 {
                let x = some!(42);
                x
            }
        "#})
        .dep("invalid_prebuilt_example", t.child("dep"))
        .manifest_extra(indoc! {r#"
            [tool.scarb]
            allow-prebuilt-plugins = ["invalid_prebuilt_example"]
        "#})
        .build(&t);
    Scarb::quick_snapbox()
        .arg("build")
        // Disable output from Cargo.
        .env("CARGO_TERM_QUIET", "true")
        .current_dir(&t)
        .assert()
        .success()
        .stdout_matches(indoc! {r#"
            [..]Compiling invalid_prebuilt_example v0.1.0 ([..])
            [..]Compiling hello v1.0.0 ([..]Scarb.toml)
            [..] Finished `dev` profile target(s) in [..]
        "#});
}
