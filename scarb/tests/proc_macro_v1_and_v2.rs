use assert_fs::TempDir;
use assert_fs::fixture::PathChild;
use indoc::indoc;
use scarb_test_support::cairo_plugin_project_builder::CairoPluginProjectBuilder;
use scarb_test_support::command::Scarb;
use scarb_test_support::project_builder::ProjectBuilder;

#[test]
fn can_use_both_v1_and_v2_proc_macros() {
    let temp = TempDir::new().unwrap();
    let foo = temp.child("foo");
    CairoPluginProjectBuilder::default_v1()
        .name("foo")
        .lib_rs(indoc! {r##"
        use cairo_lang_macro::{ProcMacroResult, TokenStream, attribute_macro};

        #[attribute_macro]
        pub fn foo(_attr: TokenStream, token_stream: TokenStream) -> ProcMacroResult {
            let token_stream = TokenStream::new(
                token_stream
                    .to_string()
                    .replace("12", "34")
            );
            ProcMacroResult::new(token_stream)
        }
        "##})
        .build(&foo);
    let bar = temp.child("bar");
    CairoPluginProjectBuilder::default()
        .name("bar")
        .lib_rs(indoc! {r##"
        use cairo_lang_macro::{ProcMacroResult, TokenStream, attribute_macro, TokenTree, Token, TextSpan};

        #[attribute_macro]
        pub fn bar(_attr: TokenStream, token_stream: TokenStream) -> ProcMacroResult {
            let token_stream = TokenStream::new(
                token_stream
                    .into_iter()
                    .map(|TokenTree::Ident(token)| {
                        if token.content.to_string() == "12" {
                            TokenTree::Ident(Token::new("34", TextSpan::call_site()))
                        } else {
                            TokenTree::Ident(token)
                        }
                    })
                    .collect(),
            );
            ProcMacroResult::new(token_stream)
        }
        "##})
        .build(&bar);
    let project = temp.child("hello");
    ProjectBuilder::start()
        .name("hello")
        .version("1.0.0")
        .dep("foo", &foo)
        .dep("bar", &bar)
        .dep_cairo_execute()
        .manifest_extra(indoc! {r#"
            [executable]

            [cairo]
            enable-gas = false
        "#})
        .lib_cairo(indoc! {r#"
            #[foo]
            fn first() -> felt252 {12}

            #[bar]
            fn second() -> felt252 {12}

            #[executable]
            fn main() -> felt252 { first() + second() }
        "#})
        .build(&project);

    Scarb::quick_command()
        .arg("execute")
        .arg("--print-program-output")
        // Disable output from Cargo.
        .env("CARGO_TERM_QUIET", "true")
        .current_dir(&project)
        .assert()
        .success()
        .stdout_eq(indoc! {r#"
            [..]Compiling bar v1.0.0 ([..]Scarb.toml)
            [..]Compiling foo v1.0.0 ([..]Scarb.toml)
            [..] Compiling hello v1.0.0 ([..]Scarb.toml)
            [..]Finished `dev` profile target(s) in [..]
            [..]Executing hello
            Program output:
            68
            "#});
}

#[test]
fn v1_and_v2_macros_cannot_duplicate_expansions() {
    let temp = TempDir::new().unwrap();
    let foo = temp.child("foo");
    CairoPluginProjectBuilder::default_v1()
        .name("foo")
        .lib_rs(indoc! {r##"
        use cairo_lang_macro::{ProcMacroResult, TokenStream, attribute_macro};

        #[attribute_macro]
        pub fn some(_attr: TokenStream, token_stream: TokenStream) -> ProcMacroResult {
            let token_stream = TokenStream::new(
                token_stream
                    .to_string()
                    .replace("12", "34")
            );
            ProcMacroResult::new(token_stream)
        }
        "##})
        .build(&foo);
    let bar = temp.child("bar");
    CairoPluginProjectBuilder::default()
        .name("bar")
        .lib_rs(indoc! {r##"
        use cairo_lang_macro::{ProcMacroResult, TokenStream, attribute_macro, TokenTree, Token, TextSpan};

        #[attribute_macro]
        pub fn some(_attr: TokenStream, token_stream: TokenStream) -> ProcMacroResult {
            let token_stream = TokenStream::new(
                token_stream
                    .into_iter()
                    .map(|TokenTree::Ident(token)| {
                        if token.content.to_string() == "12" {
                            TokenTree::Ident(Token::new("34", TextSpan::call_site()))
                        } else {
                            TokenTree::Ident(token)
                        }
                    })
                    .collect(),
            );
            ProcMacroResult::new(token_stream)
        }
        "##})
        .build(&bar);
    let project = temp.child("hello");
    ProjectBuilder::start()
        .name("hello")
        .version("1.0.0")
        .dep("foo", &foo)
        .dep("bar", &bar)
        .lib_cairo(indoc! {r#"
            #[some]
            fn main() -> felt252 {12}
        "#})
        .build(&project);

    Scarb::quick_command()
        .arg("build")
        // Disable output from Cargo.
        .env("CARGO_TERM_QUIET", "true")
        .current_dir(&project)
        .assert()
        .failure()
        .stdout_eq(indoc! {r#"
            [..]Compiling bar v1.0.0 ([..]Scarb.toml)
            [..]Compiling foo v1.0.0 ([..]Scarb.toml)
            [..]Compiling hello v1.0.0 ([..]Scarb.toml)
            error: duplicate expansions defined for procedural macros: some (bar v1.0.0 ([..]Scarb.toml) and foo v1.0.0 ([..]Scarb.toml))
            error: could not compile `hello` due to [..] previous error[..]
        "#});
}
