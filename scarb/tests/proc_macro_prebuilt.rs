use assert_fs::prelude::PathChild;
use assert_fs::TempDir;
use cairo_lang_macro::TokenStream;
use indoc::indoc;
use scarb_proc_macro_server_types::methods::expand::{ExpandInline, ExpandInlineMacroParams};
use scarb_test_support::command::Scarb;
use scarb_test_support::proc_macro_server::ProcMacroClient;
use scarb_test_support::project_builder::{Dep, DepBuilder, ProjectBuilder};
use scarb_test_support::workspace_builder::WorkspaceBuilder;

#[test]
fn compile_with_prebuilt_plugins() {
    let t = TempDir::new().unwrap();
    let builder = |name: &str| {
        ProjectBuilder::start()
            .name(name)
            .lib_cairo(indoc! {r#"
                fn main() -> u32 {
                    let x = some!(42);
                    x
                }
            "#})
            .dep(
                "proc_macro_example",
                Dep.version("0.1.2").registry("https://scarbs.dev/"),
            )
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
            [..]Downloading proc_macro_example v0.1.2 ([..])
            [..]Compiling a v1.0.0 ([..]Scarb.toml)
            [..]Compiling b v1.0.0 ([..]Scarb.toml)
            [..] Finished `dev` profile target(s) in [..]
        "#});
}

#[test]
fn compile_with_prebuilt_plugins_only_one_allows() {
    let t = TempDir::new().unwrap();
    let builder = |name: &str, allow: bool| {
        let b = ProjectBuilder::start()
            .name(name)
            .lib_cairo(indoc! {r#"
                fn main() -> u32 {
                    let x = some!(42);
                    x
                }
            "#})
            .dep(
                "proc_macro_example",
                Dep.version("0.1.2").registry("https://scarbs.dev/"),
            );
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
            [..]Downloading proc_macro_example v0.1.2 ([..])
            [..]Compiling proc_macro_example v0.1.2 (registry+https://scarbs.dev/)
            [..]Compiling a v1.0.0 ([..]Scarb.toml)
            warn: package name differs between Cargo and Scarb manifest
            cargo: `some_macro`, scarb: `proc_macro_example`
            this might become an error in future Scarb releases
            
            [..]Compiling b v1.0.0 ([..]Scarb.toml)
            warn: package name differs between Cargo and Scarb manifest
            cargo: `some_macro`, scarb: `proc_macro_example`
            this might become an error in future Scarb releases
            
            [..] Finished `dev` profile target(s) in [..]
        "#});
}

#[test]
fn compile_with_invalid_prebuilt_plugins() {
    let t = TempDir::new().unwrap();

    ProjectBuilder::start()
        .name("hello")
        .lib_cairo(indoc! {r#"
            fn main() -> u32 {
                let x = some!(42);
                x
            }
        "#})
        .dep(
            "invalid_prebuilt_example",
            Dep.version("0.1.0").registry("https://scarbs.dev/"),
        )
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
            [..]Downloading invalid_prebuilt_example v0.1.0 ([..])
            [..]Compiling invalid_prebuilt_example v0.1.0 ([..])
            [..]Compiling hello v1.0.0 ([..]Scarb.toml)
            [..] Finished `dev` profile target(s) in [..]
        "#});
}

#[test]
fn load_prebuilt_proc_macros() {
    let t = TempDir::new().unwrap();

    let project = t.child("test_package");

    ProjectBuilder::start()
        .name("test_package")
        .version("1.0.0")
        .lib_cairo("")
        .dep(
            "proc_macro_example",
            Dep.version("0.1.2").registry("https://scarbs.dev/"),
        )
        .manifest_extra(indoc! {r#"
            [tool.scarb]
            allow-prebuilt-plugins = ["proc_macro_example"]
        "#})
        .build(&project);

    let mut proc_macro_server = ProcMacroClient::new_without_cargo(&project);

    let response = proc_macro_server
        .request_and_wait::<ExpandInline>(ExpandInlineMacroParams {
            name: "some".to_string(),
            args: TokenStream::new("42".to_string()),
        })
        .unwrap();

    assert_eq!(response.diagnostics, vec![]);
    assert_eq!(response.token_stream, TokenStream::new("42".to_string()));
}
