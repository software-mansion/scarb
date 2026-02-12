use assert_fs::TempDir;
use assert_fs::prelude::PathChild;
use indoc::indoc;
use scarb_metadata::Metadata;
use scarb_test_support::cairo_plugin_project_builder::CairoPluginProjectBuilder;
use scarb_test_support::command::{CommandExt, Scarb};
use scarb_test_support::project_builder::ProjectBuilder;

#[test]
fn cairo_plugin_re_export_simple() {
    let t = TempDir::new().unwrap();
    CairoPluginProjectBuilder::default()
        .name("world")
        .build(&t.child("world"));
    ProjectBuilder::start()
        .name("beautiful")
        .dep("world", t.child("world"))
        .manifest_package_extra(indoc! {r#"
            re-export-cairo-plugins = ["world"]
        "#})
        .build(&t.child("beautiful"));
    ProjectBuilder::start()
        .name("hello")
        .dep("beautiful", t.child("beautiful"))
        .dep("world", t.child("world"))
        .build(&t.child("hello"));

    let meta = Scarb::quick_command()
        .arg("--json")
        .arg("metadata")
        .arg("--format-version")
        .arg("1")
        .current_dir(t.child("hello"))
        .stdout_json::<Metadata>();
    let hello_cu = meta
        .compilation_units
        .iter()
        .find(|cu| cu.target.name == "hello" && cu.target.kind == "lib")
        .unwrap();

    assert_eq!(
        hello_cu.cairo_plugins.len(),
        1,
        "CU should contain exactly one plugin"
    );
    assert!(
        hello_cu
            .cairo_plugins
            .first()
            .unwrap()
            .package
            .to_string()
            .starts_with("world 1.0.0"),
        "plugin world not found in cu plugins"
    );
    assert_eq!(
        hello_cu.components.len(),
        3,
        "CU should contain exactly three components"
    );
    let beautiful_component = hello_cu
        .components
        .iter()
        .find(|c| c.name == "beautiful")
        .unwrap();
    assert_eq!(
        beautiful_component.dependencies.clone().unwrap().len(),
        3,
        "beautiful component should have 3 dependencies"
    );

    Scarb::quick_command()
        .arg("build")
        // Disable output from Cargo.
        .env("CARGO_TERM_QUIET", "true")
        .current_dir(t.child("hello"))
        .assert()
        .success()
        .stdout_eq(indoc! {r#"
           [..]Compiling world v1.0.0 [..]
           [..]Compiling hello v1.0.0 [..]
           [..]Finished `dev` profile target(s) in [..]
        "#});
}

#[test]
#[ignore]
fn components_in_the_same_unit_can_depend_on_conflicting_plugins() {
    let t = TempDir::new().unwrap();
    CairoPluginProjectBuilder::default()
        .name("first_macro")
        .lib_rs(indoc! {r##"
        use cairo_lang_macro::{ProcMacroResult, TokenStream, attribute_macro, TokenTree, Token, TextSpan};

        #[attribute_macro]
        pub fn some(_attr: TokenStream, token_stream: TokenStream) -> ProcMacroResult {
            let new_token_string = token_stream.to_string().replace("12", "34");
            let token_stream = TokenStream::new(vec![TokenTree::Ident(Token::new(
                new_token_string.clone(),
                TextSpan { start: 0, end: new_token_string.len() as u32 },
            ))]);
            ProcMacroResult::new(token_stream)
        }
        "##})
        .build(&t.child("first_macro"));
    CairoPluginProjectBuilder::default()
        .name("second_macro")
        .lib_rs(indoc! {r##"
        use cairo_lang_macro::{ProcMacroResult, TokenStream, attribute_macro, TokenTree, Token, TextSpan};

        #[attribute_macro]
        pub fn some(_attr: TokenStream, token_stream: TokenStream) -> ProcMacroResult {
            let new_token_string = token_stream.to_string().replace("56", "78");
            let token_stream = TokenStream::new(vec![TokenTree::Ident(Token::new(
                new_token_string.clone(),
                TextSpan { start: 0, end: new_token_string.len() as u32 },
            ))]);
            ProcMacroResult::new(token_stream)
        }
        "##})
        .build(&t.child("second_macro"));
    ProjectBuilder::start()
        .name("first_dep")
        .lib_cairo(
            r#"
            #[some]
            pub fn main() -> felt252 {
                12
            }
        "#,
        )
        .dep("first_macro", t.child("first_macro"))
        .build(&t.child("first_dep"));
    ProjectBuilder::start()
        .name("second_dep")
        .lib_cairo(
            r#"
            #[some]
            pub fn main() -> felt252 {
                56
            }
        "#,
        )
        .dep("second_macro", t.child("second_macro"))
        .build(&t.child("second_dep"));
    ProjectBuilder::start()
        .name("hello")
        .lib_cairo(indoc! {r#"
            #[executable]
            pub fn main() -> felt252 {
                first_dep::main() + second_dep::main()
            }
        "#})
        .dep_cairo_execute()
        .manifest_extra(indoc! {r#"
            [executable]

            [cairo]
            enable-gas = false
        "#})
        .dep("first_dep", t.child("first_dep"))
        .dep("second_dep", t.child("second_dep"))
        .build(&t.child("hello"));
    Scarb::quick_command()
        .arg("execute")
        .arg("--print-program-output")
        // Disable output from Cargo.
        .env("CARGO_TERM_QUIET", "true")
        .current_dir(t.child("hello"))
        .assert()
        .success()
        // 112 = 34 + 78
        .stdout_eq(indoc! {r#"
            [..] Compiling first_macro v1.0.0 ([..]Scarb.toml)
            [..] Compiling second_macro v1.0.0 ([..]Scarb.toml)
            [..] Compiling hello v1.0.0 ([..]Scarb.toml)
            [..]Finished `dev` profile target(s) in [..]
            [..]Executing hello
            Program output:
            112
        "#});
}
