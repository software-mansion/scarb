use assert_fs::fixture::{ChildPath, FileWriteStr, PathCreateDir};
use assert_fs::prelude::PathChild;
use assert_fs::TempDir;
use cairo_lang_macro::{TextSpan, Token, TokenStream, TokenTree};
use indoc::indoc;
use libloading::library_filename;
use scarb_proc_macro_server_types::methods::expand::{ExpandInline, ExpandInlineMacroParams};
use scarb_test_support::cairo_plugin_project_builder::CairoPluginProjectBuilder;
use scarb_test_support::command::Scarb;
use scarb_test_support::proc_macro_server::ProcMacroClient;
use scarb_test_support::project_builder::ProjectBuilder;
use scarb_test_support::workspace_builder::WorkspaceBuilder;
use snapbox::cmd::Command;
use std::fs;

static TRIPLETS: [(&str, &str); 4] = [
    ("aarch64-apple-darwin", ".dylib"),
    ("x86_64-apple-darwin", ".dylib"),
    ("x86_64-unknown-linux-gnu", ".so"),
    ("x86_64-pc-windows-msvc", ".dll"),
];

fn proc_macro_example(t: &ChildPath) {
    let name = "proc_macro_example";
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
    let dll_filename = library_filename(name);
    let dll_filename = dll_filename.to_string_lossy().to_string();
    let build_dir = t.child("cargo_build_dir");
    Command::new("cargo")
        .arg("build")
        .arg("--release")
        .env("CARGO_TARGET_DIR", build_dir.path())
        .current_dir(t)
        .assert()
        .success();
    t.child("target/scarb/cairo-plugin")
        .create_dir_all()
        .unwrap();
    for (target, extension) in TRIPLETS {
        let target_name = format!("{name}_v{version}_{target}{extension}");
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

#[test]
fn load_prebuilt_proc_macros() {
    let t = TempDir::new().unwrap();
    proc_macro_example(&t.child("dep"));
    let project = t.child("test_package");
    ProjectBuilder::start()
        .name("test_package")
        .version("1.0.0")
        .lib_cairo("")
        .dep("proc_macro_example", t.child("dep"))
        .manifest_extra(indoc! {r#"
            [tool.scarb]
            allow-prebuilt-plugins = ["proc_macro_example"]
        "#})
        .build(&project);
    let mut proc_macro_server = ProcMacroClient::new_without_cargo(&project);
    let response = proc_macro_server
        .request_and_wait::<ExpandInline>(ExpandInlineMacroParams {
            name: "some".to_string(),
            args: TokenStream::new(vec![TokenTree::Ident(Token::new(
                "42",
                TextSpan::call_site(),
            ))]),
            call_site: TextSpan::new(0, 0),
        })
        .unwrap();
    assert_eq!(response.diagnostics, vec![]);
    assert_eq!(
        response.token_stream,
        TokenStream::new(vec![TokenTree::Ident(Token::new(
            "42",
            TextSpan::call_site(),
        ))])
    );
}
