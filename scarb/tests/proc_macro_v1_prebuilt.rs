use assert_fs::fixture::{ChildPath, PathCreateDir};
use assert_fs::prelude::PathChild;
use assert_fs::TempDir;
use cairo_lang_macro_v2::{TextSpan, Token, TokenStream as TokenStreamV2, TokenTree};

use indoc::indoc;
use libloading::library_filename;
use scarb_proc_macro_server_types::methods::expand::{ExpandInline, ExpandInlineMacroParams};
use scarb_proc_macro_server_types::scope::ProcMacroScope;
use scarb_test_support::cairo_plugin_project_builder::CairoPluginProjectBuilder;
use scarb_test_support::proc_macro_server::{DefinedMacrosInfo, ProcMacroClient};
use scarb_test_support::project_builder::ProjectBuilder;
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
    CairoPluginProjectBuilder::default_v1()
        .name(name)
        .version(version)
        .lib_rs(indoc! {r#"
            use cairo_lang_macro_v2::{ProcMacroResult, TokenStream, inline_macro};
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

    let mut proc_macro_client = ProcMacroClient::new_without_cargo(&project);

    let DefinedMacrosInfo {
        package_id: compilation_unit_main_component_id,
        ..
    } = proc_macro_client.defined_macros_for_package("test_package");

    let response = proc_macro_client
        .request_and_wait::<ExpandInline>(ExpandInlineMacroParams {
            context: ProcMacroScope {
                package_id: compilation_unit_main_component_id,
            },
            name: "some".to_string(),
            args: TokenStreamV2::new(vec![TokenTree::Ident(Token::new(
                "42",
                TextSpan::new(0, 0),
            ))]),
            call_site: TextSpan::new(0, 0),
        })
        .unwrap();

    assert_eq!(response.diagnostics, vec![]);
    assert_eq!(
        response.token_stream,
        TokenStreamV2::new(vec![TokenTree::Ident(Token::new(
            "42",
            TextSpan::new(0, 0),
        ))])
    );
}
