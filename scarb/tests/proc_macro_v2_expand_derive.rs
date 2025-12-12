use assert_fs::TempDir;
use assert_fs::fixture::PathChild;
use indoc::indoc;
use scarb_test_support::cairo_plugin_project_builder::CairoPluginProjectBuilder;
use scarb_test_support::command::Scarb;
use scarb_test_support::fsx::ChildPathEx;
use scarb_test_support::project_builder::ProjectBuilder;
use snapbox::Assert;

#[test]
fn can_implement_derive_macro() {
    let temp = TempDir::new().unwrap();
    let t = temp.child("some");
    CairoPluginProjectBuilder::default()
        .lib_rs(indoc! {r##"
            use cairo_lang_macro::{derive_macro, ProcMacroResult, TokenStream, TokenTree, Token, TextSpan};

            #[derive_macro]
            pub fn custom_derive_v2(token_stream: TokenStream) -> ProcMacroResult {
                let name = token_stream
                    .clone()
                    .to_string()
                    .lines()
                    .find(|l| l.starts_with("struct"))
                    .unwrap()
                    .to_string()
                    .replace("struct", "")
                    .replace("}", "")
                    .replace("{", "")
                    .trim()
                    .to_string();

                let code = indoc::formatdoc!{r#"
                    impl SomeImpl of Hello<{name}> {{
                        fn world(self: @{name}) -> u32 {{
                            32
                        }}
                    }}
                "#};

                let token_stream = TokenStream::new(vec![TokenTree::Ident(Token::new(
                  code.clone(),
                    TextSpan {
                        start: 0,
                        end: code.len() as u32,
                    },
                ))]);

                ProcMacroResult::new(token_stream)
            }
        "##})
        .add_dep(r#"indoc = "*""#)
        .build(&t);

    let project = temp.child("hello");
    ProjectBuilder::start()
        .name("hello")
        .version("1.0.0")
        .dep("some", &t)
        .dep_cairo_execute()
        .manifest_extra(indoc! {r#"
            [executable]

            [cairo]
            enable-gas = false
        "#})
        .lib_cairo(indoc! {r#"
            trait Hello<T> {
                fn world(self: @T) -> u32;
            }

            #[derive(CustomDeriveV2, Drop)]
            struct SomeType {}

            #[executable]
            fn main() -> u32 {
                let a = SomeType {};
                a.world()
            }
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
            [..] Compiling some v1.0.0 ([..]Scarb.toml)
            [..] Compiling hello v1.0.0 ([..]Scarb.toml)
            [..]Finished `dev` profile target(s) in [..]
            [..]Executing hello
            Program output:
            32
            Saving output to: target/execute/hello/execution1
        "#});
}

#[test]
fn can_use_both_derive_and_attr() {
    let temp = TempDir::new().unwrap();
    let t = temp.child("some");
    CairoPluginProjectBuilder::default()
        .lib_rs(indoc! {r##"
            use cairo_lang_macro::{derive_macro, attribute_macro, ProcMacroResult, TokenStream, TokenTree, TextSpan, Token};

            #[attribute_macro]
            pub fn first_attribute(_attr: TokenStream, token_stream: TokenStream) -> ProcMacroResult {
                let new_token_string = token_stream.to_string().replace("SomeType", "OtherType");
                ProcMacroResult::new(TokenStream::new(vec![TokenTree::Ident(Token::new(
                  new_token_string.clone(),
                    TextSpan {
                        start: 0,
                        end: new_token_string.len() as u32,
                    },
                ))]))
            }

            #[attribute_macro]
            pub fn second_attribute(_attr: TokenStream, token_stream: TokenStream) -> ProcMacroResult {
                let code = token_stream.to_string().replace("OtherType", "RenamedStruct");
                let token_stream = TokenStream::new(vec![TokenTree::Ident(Token::new(
                  code.clone(),
                    TextSpan {
                        start: 0,
                        end: code.len() as u32,
                    },
                ))]);

                let result_string = format!("#[derive(Drop)]\n{token_stream}");
                ProcMacroResult::new(TokenStream::new(vec![TokenTree::Ident(Token::new(
                  result_string.clone(),
                    TextSpan {
                        start: 0,
                        end: result_string.len() as u32,
                    },
                ))]))
            }

            #[derive_macro]
            pub fn custom_derive(_token_stream: TokenStream) -> ProcMacroResult {
                let code = indoc::formatdoc!{r#"
                    impl SomeImpl of Hello<RenamedStruct> {{
                        fn world(self: @RenamedStruct) -> u32 {{
                            32
                        }}
                    }}
                    "#};

                ProcMacroResult::new(TokenStream::new(vec![TokenTree::Ident(Token::new(
                  code.clone(),
                    TextSpan {
                        start: 0,
                        end: code.len() as u32,
                    },
                ))]))
            }
        "##})
        .add_dep(r#"indoc = "*""#)
        .build(&t);

    let project = temp.child("hello");
    ProjectBuilder::start()
        .name("hello")
        .version("1.0.0")
        .dep("some", &t)
        .dep_cairo_execute()
        .manifest_extra(indoc! {r#"
            [executable]

            [cairo]
            enable-gas = false
        "#})
        .lib_cairo(indoc! {r#"
            trait Hello<T> {
                fn world(self: @T) -> u32;
            }

            #[first_attribute]
            #[derive(CustomDerive)]
            #[second_attribute]
            struct SomeType {}

            #[executable]
            fn main() -> u32 {
                let a = RenamedStruct {};
                a.world()
            }
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
            [..] Compiling some v1.0.0 ([..]Scarb.toml)
            [..] Compiling hello v1.0.0 ([..]Scarb.toml)
            [..]Finished `dev` profile target(s) in [..]
            [..]Executing hello
            Program output:
            32
            Saving output to: target/execute/hello/execution1
        "#});
}

#[test]
fn can_be_expanded() {
    let temp = TempDir::new().unwrap();
    let t = temp.child("some");
    CairoPluginProjectBuilder::default()
        .lib_rs(indoc! {r##"
        use cairo_lang_macro::{ProcMacroResult, TokenStream, attribute_macro, derive_macro, TokenTree, Token, TextSpan};

        #[attribute_macro]
        pub fn some(_attr: TokenStream, token_stream: TokenStream) -> ProcMacroResult {
            let new_token_string = token_stream.to_string().replace("12", "34");
            let token_stream = TokenStream::new(vec![TokenTree::Ident(Token::new(
                new_token_string.clone(),
                TextSpan { start: 0, end: new_token_string.len() as u32 },
            ))]);
            ProcMacroResult::new(token_stream)
        }

        #[derive_macro]
        pub fn custom_derive(token_stream: TokenStream) -> ProcMacroResult {
            let name = token_stream
                .clone()
                .to_string()
                .lines()
                .find(|l| l.starts_with("struct"))
                .unwrap()
                .to_string()
                .replace("struct", "")
                .replace("}", "")
                .replace("{", "")
                .trim()
                .to_string();

            let code = indoc::formatdoc!{r#"
                impl SomeImpl of Hello<{name}> {{
                    fn world(self: @{name}) -> u32 {{
                        32
                    }}
                }}
            "#};

            let token_stream = TokenStream::new(vec![TokenTree::Ident(Token::new(
                code.clone(),
                TextSpan { start: 0, end: code.len() as u32 },
            ))]);

            ProcMacroResult::new(token_stream)
        }
        "##})
        .add_dep(r#"indoc = "*""#)
        .build(&t);
    let project = temp.child("hello");
    ProjectBuilder::start()
        .name("hello")
        .version("1.0.0")
        .dep("some", &t)
        .lib_cairo(indoc! {r#"
            trait Hello<T> {
                fn world(self: @T) -> u32;
            }

            #[derive(CustomDerive, Drop)]
            struct SomeType {}

            #[some]
            fn main() -> u32 {
                let x = 12;
                let a = SomeType {};
                a.world() + x
            }
        "#})
        .build(&project);

    Scarb::quick_command()
        .arg("expand")
        // Disable output from Cargo.
        .env("CARGO_TERM_QUIET", "true")
        .current_dir(&project)
        .assert()
        .success();

    assert_eq!(
        project.child("target/dev").files(),
        vec!["hello.expanded.cairo"]
    );
    let expanded = project
        .child("target/dev/hello.expanded.cairo")
        .read_to_string();
    Assert::new().eq(
        expanded,
        indoc! {r#"
        mod hello {
            trait Hello<T> {
                fn world(self: @T) -> u32;
            }

            #[derive(CustomDerive, Drop)]
            struct SomeType {}
            impl SomeImpl of Hello<SomeType> {
                fn world(self: @SomeType) -> u32 {
                    32
                }
            }
            impl SomeTypeDrop<> of core::traits::Drop<SomeType>;
            fn main() -> u32 {
                let x = 34;
                let a = SomeType {};
                a.world() + x
            }
        }
        "#},
    );
}

#[test]
fn code_mappings_preserve_derive_error_locations() {
    let temp = TempDir::new().unwrap();
    let t = temp.child("some");
    CairoPluginProjectBuilder::default()
        .lib_rs(indoc! {r##"
            use cairo_lang_macro::{derive_macro, ProcMacroResult, TokenStream, TokenTree, Token};

            #[derive_macro]
            pub fn custom_derive(token_stream: TokenStream) -> ProcMacroResult {
                let name = token_stream
                    .clone()
                    .to_string()
                    .lines()
                    .find(|l| l.starts_with("struct"))
                    .unwrap()
                    .to_string()
                    .replace("struct", "")
                    .replace("}", "")
                    .replace("{", "")
                    .trim()
                    .to_string();

                let code = indoc::formatdoc!{r#"
                    impl SomeImpl{name} of Hello<{name}> {{
                        fn world(self: @{name}) -> u8 {{
                            256
                        }}
                    }}
                "#};

                let second_token_span = match &token_stream.tokens[1] {
                    TokenTree::Ident(t) => t.span.clone(),
                };

                let token_stream = TokenStream::new(vec![TokenTree::Ident(Token::new(
                  code.clone(), second_token_span
                ))]);

                ProcMacroResult::new(token_stream)
            }
        "##})
        .add_dep(r#"indoc = "*""#)
        .build(&t);

    let project = temp.child("hello");
    ProjectBuilder::start()
        .name("hello")
        .version("1.0.0")
        .dep("some", &t)
        .lib_cairo(indoc! {r#"
            trait Hello<T> {
                fn world(self: @T) -> u8;
            }

            #[derive(CustomDerive, Drop)]
            struct SomeType {}

            #[derive(CustomDerive, Drop)]
            struct AnotherType {}

            fn main() -> u8 {
                let a = SomeType {};
                a.world()
            }
        "#})
        .build(&project);

    Scarb::quick_command()
        .arg("build")
        .env("CARGO_TERM_QUIET", "true")
        .current_dir(&project)
        .assert()
        .failure()
        .stdout_eq(indoc! {r#"
            [..]Compiling some v1.0.0 ([..]Scarb.toml)
            [..]Compiling hello v1.0.0 ([..]Scarb.toml)
            error: The value does not fit within the range of type core::integer::u8.
             --> [..]lib.cairo:5:1
            #[derive(CustomDerive, Drop)]
            ^
            note: this error originates in the derive macro: `CustomDerive`

            error: The value does not fit within the range of type core::integer::u8.
             --> [..]lib.cairo:8:1
            #[derive(CustomDerive, Drop)]
            ^
            note: this error originates in the derive macro: `CustomDerive`

            error: could not compile `hello` due to [..] previous error[..]
        "#});
}

#[test]
fn diags_can_be_mapped_to_call_site_correctly() {
    let temp = TempDir::new().unwrap();
    let t = temp.child("some");
    CairoPluginProjectBuilder::default()
        .lib_rs(indoc! {r#"
        use cairo_lang_macro::{derive_macro, quote, ProcMacroResult, TokenStream};

        #[derive_macro]
        pub fn improper_derive_macro_v2(_item: TokenStream) -> ProcMacroResult {
            let ts = quote! {
                fn generated_function_v2() {
                    some <$> syntax
                }
            };
            ProcMacroResult::new(ts)
        }
        "#})
        .build(&t);
    let project = temp.child("hello");
    ProjectBuilder::start()
        .name("hello")
        .version("1.0.0")
        .dep("some", &t)
        .lib_cairo(indoc! {r#"
            #[derive(ImproperDeriveMacroV2)]
            struct X {}
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
            [..] Compiling some v1.0.0 ([..]Scarb.toml)
            [..] Compiling hello v1.0.0 ([..]Scarb.toml)
            error: Missing tokens. Expected a path segment.
             --> [..]lib.cairo:1:10
            #[derive(ImproperDeriveMacroV2)]
                     ^^^^^^^^^^^^^^^^^^^^^
            note: this error originates in the derive macro: `ImproperDeriveMacroV2`

            error: Consecutive comparison operators are not allowed: '<' followed by '>'
             --> [..]lib.cairo:1:10
            #[derive(ImproperDeriveMacroV2)]
                     ^^^^^^^^^^^^^^^^^^^^^
            note: this error originates in the derive macro: `ImproperDeriveMacroV2`

            error[E0006]: Identifier not found.
             --> [..]lib.cairo:1:10
            #[derive(ImproperDeriveMacroV2)]
                     ^^^^^^^^^^^^^^^^^^^^^
            note: this error originates in the derive macro: `ImproperDeriveMacroV2`

            error: Are you missing a `::`?.
             --> [..]lib.cairo:1:10
            #[derive(ImproperDeriveMacroV2)]
                     ^^^^^^^^^^^^^^^^^^^^^
            note: this error originates in the derive macro: `ImproperDeriveMacroV2`

            error: could not compile `hello` due to [..] previous error[..]
    "#});
}
#[test]
fn can_use_two_derive_macros() {
    let temp = TempDir::new().unwrap();
    let t = temp.child("some");
    CairoPluginProjectBuilder::default()
        .lib_rs(indoc! {r##"
            use cairo_lang_macro::{derive_macro, ProcMacroResult, TokenStream, TokenTree, Token, TextSpan};

            #[derive_macro]
            pub fn custom_derive_v2(token_stream: TokenStream) -> ProcMacroResult {
                let name = token_stream
                    .clone()
                    .to_string()
                    .lines()
                    .find(|l| l.starts_with("struct"))
                    .unwrap()
                    .to_string()
                    .replace("struct", "")
                    .replace("}", "")
                    .replace("{", "")
                    .trim()
                    .to_string();

                let code = indoc::formatdoc!{r#"
                    impl SomeImpl{name} of CustomTrait<{name}> {{
                        fn custom(self: @{name}) -> u32 {{
                            32
                        }}
                    }}
                "#};

                let token_stream = TokenStream::new(vec![TokenTree::Ident(Token::new(
                  code.clone(),
                  TextSpan::call_site(),
                ))]);

                ProcMacroResult::new(token_stream)
            }

            #[derive_macro]
            pub fn my_derive_v2(token_stream: TokenStream) -> ProcMacroResult {
                let name = token_stream
                    .clone()
                    .to_string()
                    .lines()
                    .find(|l| l.starts_with("struct"))
                    .unwrap()
                    .to_string()
                    .replace("struct", "")
                    .replace("}", "")
                    .replace("{", "")
                    .trim()
                    .to_string();

                let code = indoc::formatdoc!{r#"
                    impl MyImpl{name} of MyTrait<{name}> {{
                        fn my(self: @{name}) -> u32 {{
                            32
                        }}
                    }}
                "#};

                let token_stream = TokenStream::new(vec![TokenTree::Ident(Token::new(
                  code.clone(),
                  TextSpan::call_site(),
                ))]);

                ProcMacroResult::new(token_stream)
            }
        "##})
        .add_dep(r#"indoc = "*""#)
        .build(&t);

    let project = temp.child("hello");
    ProjectBuilder::start()
        .name("hello")
        .version("1.0.0")
        .dep("some", &t)
        .dep_cairo_execute()
        .manifest_extra(indoc! {r#"
            [executable]

            [cairo]
            enable-gas = false
        "#})
        .lib_cairo(indoc! {r#"
            trait CustomTrait<T> {
              fn custom(self: @T) -> u32;
            }

            #[derive(CustomDeriveV2, MyDeriveV2, Drop)]
            struct SomeStruct {}

            trait MyTrait<T> {
              fn my(self: @T) -> u32;
            }

            #[executable]
            fn main() -> u32 {
                let a = SomeStruct {};
                assert(a.custom() == a.my(), '');
                a.custom()
            }
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
            [..] Compiling some v1.0.0 ([..]Scarb.toml)
            [..] Compiling hello v1.0.0 ([..]Scarb.toml)
            [..]Finished `dev` profile target(s) in [..]
            [..]Executing hello
            Program output:
            32
            Saving output to: target/execute/hello/execution1
        "#});
}

#[test]
fn derive_cannot_have_module_path() {
    let temp = TempDir::new().unwrap();
    let t = temp.child("some");
    CairoPluginProjectBuilder::default()
        .lib_rs(indoc! {r##"
            use cairo_lang_macro::{derive_macro, ProcMacroResult, TokenStream};

            #[derive_macro]
            pub fn custom_derive_v2(_token_stream: TokenStream) -> ProcMacroResult {
                ProcMacroResult::new(TokenStream::empty())
            }
        "##})
        .build(&t);

    let project = temp.child("hello");
    ProjectBuilder::start()
        .name("hello")
        .version("1.0.0")
        .dep("some", &t)
        .lib_cairo(indoc! {r#"
            trait Hello<T> {
                fn world(self: @T) -> u32;
            }

            #[derive(not::a::path::CustomDeriveV2, Drop)]
            struct SomeType {}
        "#})
        .build(&project);

    Scarb::quick_command()
        .arg("check")
        // Disable output from Cargo.
        .env("CARGO_TERM_QUIET", "true")
        .current_dir(&project)
        .assert()
        .failure()
        .stdout_eq(indoc! {r#"
            [..] Compiling some v1.0.0 ([..]Scarb.toml)
            [..] Checking hello v1.0.0 ([..]Scarb.toml)
            error: Plugin diagnostic: Unknown derive `not::a::path::CustomDeriveV2` - a plugin might be missing.
             --> [..]lib.cairo:5:10
            #[derive(not::a::path::CustomDeriveV2, Drop)]
                     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^

            error: could not check `hello` due to 1 previous error
        "#});
}
