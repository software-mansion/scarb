use assert_fs::TempDir;
use assert_fs::fixture::PathChild;
use assert_fs::prelude::*;
use indoc::indoc;
use scarb_test_support::cairo_plugin_project_builder::CairoPluginProjectBuilder;
use scarb_test_support::command::Scarb;
use scarb_test_support::fsx::ChildPathEx;
use scarb_test_support::project_builder::ProjectBuilder;
use scarb_test_support::workspace_builder::WorkspaceBuilder;
use snapbox::Assert;

#[test]
fn can_emit_plugin_warning() {
    let temp = TempDir::new().unwrap();
    let t = temp.child("some");
    CairoPluginProjectBuilder::default()
        .lib_rs(indoc! {r#"
        use cairo_lang_macro::{ProcMacroResult, TokenStream, attribute_macro, Diagnostic};

        #[attribute_macro]
        pub fn some(_attr: TokenStream, token_stream: TokenStream) -> ProcMacroResult {
            let diag = Diagnostic::warn("Some warning from macro.");
            ProcMacroResult::new(token_stream)
                .with_diagnostics(diag.into())
        }
        "#})
        .build(&t);
    let project = temp.child("hello");
    ProjectBuilder::start()
        .name("hello")
        .version("1.0.0")
        .dep("some", &t)
        .lib_cairo(indoc! {r#"
            #[some]
            fn f() -> felt252 { 12 }
        "#})
        .build(&project);

    Scarb::quick_snapbox()
        .arg("build")
        // Disable output from Cargo.
        .env("CARGO_TERM_QUIET", "true")
        .current_dir(&project)
        .assert()
        .success()
        .stdout_eq(indoc! {r#"
            [..] Compiling some v1.0.0 ([..]Scarb.toml)
            [..] Compiling hello v1.0.0 ([..]Scarb.toml)
            warn: Plugin diagnostic: Some warning from macro.
             --> [..]lib.cairo:1:1
            #[some]
            ^^^^^^^

            [..]Finished `dev` profile target(s) in [..]
        "#});
}

#[test]
fn diags_from_generated_code_mapped_correctly() {
    let temp = TempDir::new().unwrap();
    let t = temp.child("some");
    CairoPluginProjectBuilder::default()
        .lib_rs(indoc! {r#"
        use cairo_lang_macro::{ProcMacroResult, TokenStream, attribute_macro, Diagnostic};

        #[attribute_macro]
        pub fn some(_attr: TokenStream, token_stream: TokenStream) -> ProcMacroResult {
            let diag = Diagnostic::error("Some error from macro.");
            ProcMacroResult::new(token_stream)
                 .with_diagnostics(diag.into())
        }
        "#})
        .build(&t);
    let project = temp.child("hello");
    ProjectBuilder::start()
        .name("hello")
        .version("1.0.0")
        .dep("some", &t)
        .lib_cairo(indoc! {r#"
        #[cfg(target: 'lib')]
        #[some]
        fn test_increase_balance() {
            i_don_exist();
        }
        "#})
        .build(&project);

    Scarb::quick_snapbox()
        .arg("build")
        // Disable output from Cargo.
        .env("CARGO_TERM_QUIET", "true")
        .current_dir(&project)
        .assert()
        .failure()
        .stdout_eq(indoc! {r#"
            [..] Compiling some v1.0.0 ([..]Scarb.toml)
            [..] Compiling hello v1.0.0 ([..]Scarb.toml)
            error: Plugin diagnostic: Some error from macro.
             --> [..]lib.cairo:2:1
            #[some]
            ^^^^^^^

            error[E0006]: Function not found.
             --> [..]lib.cairo:4:5
                i_don_exist();
                ^^^^^^^^^^^

            error: could not compile `hello` due to [..] previous error[..]
    "#});
}

#[test]
fn can_remove_original_node() {
    let temp = TempDir::new().unwrap();
    let t = temp.child("some");
    CairoPluginProjectBuilder::default()
        .lib_rs(indoc! {r#"
        use cairo_lang_macro::{ProcMacroResult, TokenStream, attribute_macro};

        #[attribute_macro]
        pub fn some(_attr: TokenStream, _: TokenStream) -> ProcMacroResult {
            ProcMacroResult::new(TokenStream::empty())
        }
        "#})
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
            #[some]
            #[executable]
            fn main() -> felt252 { 12 }

            #[executable]
            fn main() -> felt252 { 34 }

            #[some]
            #[executable]
            fn main() -> felt252 { 56 }
        "#})
        .build(&project);

    Scarb::quick_snapbox()
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
            34
            Saving output to: target/execute/hello/execution1
        "#});
}

#[test]
fn can_replace_original_node() {
    let temp = TempDir::new().unwrap();
    let t = temp.child("some");
    CairoPluginProjectBuilder::default()
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
            #[some]
            #[executable]
            fn main() -> felt252 { 12 }
        "#})
        .build(&project);

    Scarb::quick_snapbox()
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
            34
            Saving output to: target/execute/hello/execution1
        "#});
}

#[test]
fn can_implement_inline_macro() {
    let temp = TempDir::new().unwrap();
    let t = temp.child("some");
    CairoPluginProjectBuilder::default()
        .lib_rs(indoc! {r##"
        use cairo_lang_macro::{ProcMacroResult, TokenStream, inline_macro, TokenTree, Token, TextSpan};

        #[inline_macro]
        pub fn some(token_stream: TokenStream) -> ProcMacroResult {
            assert_eq!(token_stream.to_string(), "()");
            ProcMacroResult::new(TokenStream::new(vec![TokenTree::Ident(Token::new(
              "34".to_string(),
              TextSpan {
                  start: 0,
                  end: 2,
              },
            ))]))
        }
        "##})
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
            #[executable]
            fn main() -> felt252 {
                let x = some!();
                x
            }
        "#})
        .build(&project);

    Scarb::quick_snapbox()
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
            34
            Saving output to: target/execute/hello/execution1
        "#});
}

#[test]
fn empty_inline_macro_result() {
    let temp = TempDir::new().unwrap();
    let t = temp.child("some");
    CairoPluginProjectBuilder::default()
        .lib_rs(indoc! {r##"
        use cairo_lang_macro::{ProcMacroResult, TokenStream, inline_macro};

        #[inline_macro]
        pub fn some(_token_stream: TokenStream) -> ProcMacroResult {
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
            fn main() -> felt252 {
                let _x = some!();
                12
            }
        "#})
        .build(&project);

    Scarb::quick_snapbox()
        .arg("build")
        // Disable output from Cargo.
        .env("CARGO_TERM_QUIET", "true")
        .current_dir(&project)
        .assert()
        .failure()
        .stdout_eq(indoc! {r#"
            [..] Compiling some v1.0.0 ([..]Scarb.toml)
            [..] Compiling hello v1.0.0 ([..]Scarb.toml)
            error: Inline macro `some` not found.
             --> [..]lib.cairo:2:14
                let _x = some!();
                         ^^^^^^^

            error: could not compile `hello` due to [..] previous error
        "#});
}

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

    Scarb::quick_snapbox()
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

    Scarb::quick_snapbox()
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
fn can_read_attribute_args() {
    let temp = TempDir::new().unwrap();
    let t = temp.child("some");
    CairoPluginProjectBuilder::default()
        .lib_rs(indoc! {r##"
        use cairo_lang_macro::{ProcMacroResult, TokenStream, attribute_macro};

        #[attribute_macro]
        pub fn some(attr: TokenStream, token_stream: TokenStream) -> ProcMacroResult {
            println!("{}", attr);
            ProcMacroResult::new(token_stream)
        }
        "##})
        .build(&t);

    let project = temp.child("hello");
    ProjectBuilder::start()
        .name("hello")
        .version("1.0.0")
        .dep_starknet()
        .dep("some", &t)
        .lib_cairo(indoc! {r#"
            #[some(
                first: "aaa",
                second: "bbb",
            )]
            fn main() -> felt252 { 12 }
        "#})
        .build(&project);

    Scarb::quick_snapbox()
        .arg("build")
        // Disable output from Cargo.
        .env("CARGO_TERM_QUIET", "true")
        .current_dir(&project)
        .assert()
        .success()
        .stdout_eq(indoc! {r#"
            [..]Compiling some v1.0.0 ([..]Scarb.toml)
            [..]Compiling hello v1.0.0 ([..]Scarb.toml)
            (
                first: "aaa",
                second: "bbb",
            )
            [..]Finished `dev` profile target(s) in [..]
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

    Scarb::quick_snapbox()
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
fn can_expand_trait_inner_func_attr() {
    let temp = TempDir::new().unwrap();
    let t = temp.child("some");
    CairoPluginProjectBuilder::default()
        .lib_rs(indoc! {r##"
            use cairo_lang_macro::{attribute_macro, ProcMacroResult, TokenStream, TokenTree, Token, TextSpan};

            #[attribute_macro]
            pub fn some(_attr: TokenStream, token_stream: TokenStream) -> ProcMacroResult {
                let new_token_string = token_stream.to_string()
                    .replace("hello", "world")
                    .replace("12", "34");
                ProcMacroResult::new(TokenStream::new(vec![TokenTree::Ident(Token::new(
                  new_token_string.clone(),
                  TextSpan { start: 0, end: new_token_string.len() as u32 },
                ))]))
            }
        "##})
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
                #[some]
                fn hello(self: @T) -> u32 {
                    12
                }
            }

            #[derive(Drop)]
            struct SomeStruct {}

            impl SomeImpl of Hello<SomeStruct> {}

            #[executable]
            fn main() -> u32 {
                let a = SomeStruct {};
                a.world()
            }
        "#})
        .build(&project);

    Scarb::quick_snapbox()
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
            34
            Saving output to: target/execute/hello/execution1
        "#});
}

#[test]
fn can_expand_impl_inner_func_attr() {
    let temp = TempDir::new().unwrap();
    let t = temp.child("some");
    CairoPluginProjectBuilder::default()
        .lib_rs(indoc! {r##"
            use cairo_lang_macro::{attribute_macro, ProcMacroResult, TokenStream, Token, TokenTree, TextSpan};

            #[attribute_macro]
            pub fn some(_attr: TokenStream, token_stream: TokenStream) -> ProcMacroResult {
                let new_token_string = token_stream.to_string().replace("1", "1234567890");
                ProcMacroResult::new(TokenStream::new(vec![TokenTree::Ident(Token::new(
                    new_token_string.clone(),
                    TextSpan { start: 0, end: new_token_string.len() as u32 },
                ))]))
            }
        "##})
        .build(&t);

    let project = temp.child("hello");
    ProjectBuilder::start()
        .name("hello")
        .edition("2023_01")
        .version("1.0.0")
        .dep_starknet()
        .dep_cairo_test()
        .dep("some", &t)
        .manifest_extra(indoc! {r#"
            [[target.starknet-contract]]
        "#})
        .lib_cairo(indoc! {r#"
            #[starknet::interface]
            trait IHello<T> {
                fn get(self: @T) -> u128;
                fn increase(ref self: T);
            }

            #[starknet::contract]
            mod Hello {
                use starknet::storage::{StoragePointerReadAccess, StoragePointerWriteAccess};
                use starknet::get_contract_address;
                use super::IHello;

                #[storage]
                struct Storage {
                    counter: u128
                }

                #[constructor]
                fn constructor(ref self: ContractState, value_: u128) {
                    self.counter.write(value_);
                }

                #[abi(embed_v0)]
                impl IncImpl of IHello<ContractState> {
                    fn get(self: @ContractState) -> u128 {
                        self.counter.read()
                    }

                    #[some]
                    fn increase(ref self: ContractState)  {
                        self.counter.write( self.counter.read() + 1 );
                    }
                }
            }
        "#})
        .build(&project);

    Scarb::quick_snapbox()
        .arg("build")
        // Disable output from Cargo.
        .env("CARGO_TERM_QUIET", "true")
        .current_dir(&project)
        .assert()
        .success();

    project
        .child("target/dev/hello_Hello.contract_class.json")
        .assert(predicates::str::contains("1234567890").count(2));
}

#[test]
fn code_mappings_preserve_attribute_error_on_inner_trait_locations() {
    let temp = TempDir::new().unwrap();
    let t = temp.child("some");
    CairoPluginProjectBuilder::default()
        .lib_rs(indoc! {r##"
            use cairo_lang_macro::{attribute_macro, ProcMacroResult, TokenStream, TokenTree, Token, TextSpan};

            #[attribute_macro]
            pub fn some(_attr: TokenStream, token_stream: TokenStream) -> ProcMacroResult {
                ProcMacroResult::new(TokenStream::new(
                    token_stream
                    .into_iter()
                    .map(|TokenTree::Ident(token)| {
                        if token.content.to_string() == "12" {
                            TokenTree::Ident(Token::new("34", TextSpan::call_site()))
                        } else {
                            TokenTree::Ident(token)
                        }
                    })
                    .collect()
                ))
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
                #[some]
                fn hello(self: @T) -> u32 {
                    let x = 12;
                    x = 2;
                    x
                }
            }

            #[derive(Drop)]
            struct SomeStruct {}

            impl SomeImpl of Hello<SomeStruct> {}

            fn main() -> u32 {
                let a = SomeStruct {};
                a.hello()
            }
        "#})
        .build(&project);

    Scarb::quick_snapbox()
        .arg("build")
        // Disable output from Cargo.
        .env("CARGO_TERM_QUIET", "true")
        .current_dir(&project)
        .assert()
        .failure()
        .stdout_eq(indoc! {r#"
            [..] Compiling some v1.0.0 ([..]Scarb.toml)
            [..] Compiling hello v1.0.0 ([..]Scarb.toml)
            error: Cannot assign to an immutable variable.
             --> [..]lib.cairo:5:9
                    x = 2;
                    ^^^^^
            note: this error originates in the attribute macro: `some`

            error: could not compile `hello` due to [..] previous error
        "#});
}

#[test]
fn code_mappings_preserve_attribute_error_on_inner_trait_locations_with_parser() {
    let temp = TempDir::new().unwrap();
    let t = temp.child("some");
    CairoPluginProjectBuilder::default()
        .add_cairo_lang_syntax_dep().add_cairo_lang_parser_dep().add_primitive_token_dep()
        .lib_rs(indoc! {r##"
            use cairo_lang_macro::{attribute_macro, quote, ProcMacroResult, TokenStream, TokenTree, Token, TextSpan};
            use cairo_lang_parser::utils::SimpleParserDatabase;
            use cairo_lang_syntax::node::with_db::SyntaxNodeWithDb;

            #[attribute_macro]
            pub fn some(_attr: TokenStream, token_stream: TokenStream) -> ProcMacroResult {
                let token_stream = TokenStream::new(
                    token_stream
                    .into_iter()
                    .map(|TokenTree::Ident(token)| {
                        if token.content.to_string() == "12" {
                            TokenTree::Ident(Token::new("34", TextSpan::new(0,2)))
                        } else {
                            TokenTree::Ident(token)
                        }
                    })
                    .collect()
                );
                let db_val = SimpleParserDatabase::default();
                let db = &db_val;
                let (body, _diagnostics) = db.parse_token_stream(&token_stream);
                let body = SyntaxNodeWithDb::new(&body, db);
                ProcMacroResult::new(quote!(#body))
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
                #[doc(hidden)]
                #[some]
                fn hello(self: @T) -> u32 {
                    let x = 12;
                    x = 2;
                    x
                }
            }

            #[derive(Drop)]
            struct SomeStruct {}

            impl SomeImpl of Hello<SomeStruct> {}

            fn main() -> u32 {
                let a = SomeStruct {};
                a.hello()
            }
        "#})
        .build(&project);

    Scarb::quick_snapbox()
        .arg("build")
        // Disable output from Cargo.
        .env("CARGO_TERM_QUIET", "true")
        .current_dir(&project)
        .assert()
        .failure()
        .stdout_eq(indoc! {r#"
            [..] Compiling some v1.0.0 ([..]Scarb.toml)
            [..] Compiling hello v1.0.0 ([..]Scarb.toml)
            error: Cannot assign to an immutable variable.
             --> [..]lib.cairo:6:9
                    x = 2;
                    ^^^^^
            note: this error originates in the attribute macro: `some`
            
            error: could not compile `hello` due to [..] previous error
        "#});
}

#[test]
fn can_be_used_through_re_export() {
    let temp = TempDir::new().unwrap();
    let t = temp.child("some");
    CairoPluginProjectBuilder::default()
        .lib_rs(indoc! {r##"
            use cairo_lang_macro::{ProcMacroResult, TokenStream, TokenTree, Token, TextSpan, attribute_macro};

            #[attribute_macro]
            pub fn some(_attr: TokenStream, token_stream: TokenStream) -> ProcMacroResult {
                ProcMacroResult::new(TokenStream::new(
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
                ))
            }
        "##})
        .build(&t);
    let dep = temp.child("dep");
    ProjectBuilder::start()
        .name("dep")
        .version("1.0.0")
        .dep("some", &t)
        .manifest_package_extra(indoc! {r#"
            re-export-cairo-plugins = ["some"]
        "#})
        .build(&dep);
    let project = temp.child("hello");
    ProjectBuilder::start()
        .name("hello")
        .version("1.0.0")
        .dep("dep", &dep)
        .lib_cairo(indoc! {r#"
            #[some]
            fn main() -> felt252 {12}
        "#})
        .build(&project);

    Scarb::quick_snapbox()
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
                fn main() -> felt252 {
                    34
                }
            }
        "#},
    );
}

#[test]
fn can_emit_plugin_error() {
    let temp = TempDir::new().unwrap();
    let t = temp.child("some");
    CairoPluginProjectBuilder::default()
        .lib_rs(indoc! {r#"
        use cairo_lang_macro::{ProcMacroResult, TokenStream, attribute_macro, Diagnostic};

        #[attribute_macro]
        pub fn some(_attr: TokenStream, token_stream: TokenStream) -> ProcMacroResult {
            let diag = Diagnostic::error("Some error from macro.");
            ProcMacroResult::new(token_stream)
                .with_diagnostics(diag.into())
        }
        "#})
        .build(&t);
    let project = temp.child("hello");
    ProjectBuilder::start()
        .name("hello")
        .version("1.0.0")
        .dep("some", &t)
        .lib_cairo(indoc! {r#"
            #[some]
            fn f() -> felt252 { 12 }
        "#})
        .build(&project);

    Scarb::quick_snapbox()
        .arg("build")
        // Disable output from Cargo.
        .env("CARGO_TERM_QUIET", "true")
        .current_dir(&project)
        .assert()
        .failure()
        .stdout_eq(indoc! {r#"
            [..] Compiling some v1.0.0 ([..]Scarb.toml)
            [..] Compiling hello v1.0.0 ([..]Scarb.toml)
            error: Plugin diagnostic: Some error from macro.
             --> [..]lib.cairo:1:1
            #[some]
            ^^^^^^^

            error: could not compile `hello` due to [..] previous error
        "#});
}

#[test]
fn code_mappings_preserve_attribute_error_locations() {
    let temp = TempDir::new().unwrap();
    let t = temp.child("some");
    CairoPluginProjectBuilder::default()
        .lib_rs(indoc! {r#"
        use cairo_lang_macro::{ProcMacroResult, TokenStream, attribute_macro, TokenTree, Token, TextSpan};

        #[attribute_macro]
        pub fn some(_attr: TokenStream, mut token_stream: TokenStream) -> ProcMacroResult {
            let token_stream_length = token_stream.to_string().len() as u32;
            token_stream.tokens.push(TokenTree::Ident(Token::new("    ", TextSpan { start: token_stream_length + 1, end: token_stream_length + 5 })));
            ProcMacroResult::new(token_stream)
        }
        "#})
        .build(&t);
    let project = temp.child("hello");
    ProjectBuilder::start()
        .name("hello")
        .version("1.0.0")
        .dep("some", &t)
        .lib_cairo(indoc! {r#"
            #[some]
            fn f() -> felt252 {
                let x = 1;
                x = 2;
                x
            }
        "#})
        .build(&project);

    Scarb::quick_snapbox()
        .arg("build")
        // Disable output from Cargo.
        .env("CARGO_TERM_QUIET", "true")
        .current_dir(&project)
        .assert()
        .failure()
        .stdout_eq(indoc! {r#"
            [..] Compiling some v1.0.0 ([..]Scarb.toml)
            [..] Compiling hello v1.0.0 ([..]Scarb.toml)
            error: Cannot assign to an immutable variable.
             --> [..]lib.cairo:4:5
                x = 2;
                ^^^^^
            note: this error originates in the attribute macro: `some`

            error: could not compile `hello` due to [..] previous error
        "#});
}

#[test]
fn code_mappings_preserve_inline_macro_error_locations() {
    let temp = TempDir::new().unwrap();
    let t = temp.child("some");
    CairoPluginProjectBuilder::default()
        .lib_rs(indoc! {r##"
        use cairo_lang_macro::{inline_macro, ProcMacroResult, TokenStream, TokenTree, Token, TextSpan};

        #[inline_macro]
        pub fn some(_token_stream: TokenStream) -> ProcMacroResult {
            let mut tokens = Vec::new();
            tokens.push(TokenTree::Ident(Token::new(
                "undefined".to_string(),
                TextSpan::new(0, 7),
            )));

            ProcMacroResult::new(TokenStream::new(tokens))
        }
        "##})
        .build(&t);

    let project = temp.child("hello");
    ProjectBuilder::start()
        .name("hello")
        .version("1.0.0")
        .dep("some", &t)
        .lib_cairo(indoc! {r#"
            fn main() -> felt252 {
                let _x = some!(abcdefghi);
                12
            }
        "#})
        .build(&project);

    Scarb::quick_snapbox()
        .arg("build")
        .env("CARGO_TERM_QUIET", "true")
        .current_dir(&project)
        .assert()
        .failure()
        .stdout_eq(indoc! {r#"
            [..] Compiling some v1.0.0 ([..]Scarb.toml)
            [..] Compiling hello v1.0.0 ([..]Scarb.toml)
            error[E0006]: Identifier not found.
             --> [..]lib.cairo:2:19
                let _x = some!(abcdefghi);
                              ^^^^^^^

            error: could not compile `hello` due to [..] previous error
        "#});
}

#[test]
fn inline_macro_error_on_call_site_location() {
    let temp = TempDir::new().unwrap();
    let t = temp.child("some");
    CairoPluginProjectBuilder::default()
        .lib_rs(indoc! {r##"
        use cairo_lang_macro::{inline_macro, ProcMacroResult, TokenStream, TokenTree, Token, TextSpan};

        #[inline_macro]
        pub fn some(_token_stream: TokenStream) -> ProcMacroResult {
            let mut tokens = Vec::new();
            tokens.push(TokenTree::Ident(Token::new(
                "undefined".to_string(),
                TextSpan::call_site(),
            )));

            ProcMacroResult::new(TokenStream::new(tokens))
        }
        "##})
        .build(&t);

    let project = temp.child("hello");
    ProjectBuilder::start()
        .name("hello")
        .version("1.0.0")
        .dep("some", &t)
        .lib_cairo(indoc! {r#"
            fn main() -> felt252 {
                let _x = some!(abcdefghi);
                12
            }
        "#})
        .build(&project);

    Scarb::quick_snapbox()
        .arg("build")
        .env("CARGO_TERM_QUIET", "true")
        .current_dir(&project)
        .assert()
        .failure()
        .stdout_eq(indoc! {r#"
            [..] Compiling some v1.0.0 ([..]Scarb.toml)
            [..] Compiling hello v1.0.0 ([..]Scarb.toml)
            error[E0006]: Identifier not found.
             --> [..]lib.cairo:2:14
                let _x = some!(abcdefghi);
                         ^^^^^^^^^^^^^^^^

            error: could not compile `hello` due to [..] previous error
        "#});
}

#[test]
fn inline_macro_args_can_be_parsed() {
    let temp = TempDir::new().unwrap();
    let t = temp.child("some");
    CairoPluginProjectBuilder::default()
        .add_cairo_lang_parser_dep()
        .add_cairo_lang_syntax_dep()
        .add_primitive_token_dep()
        .lib_rs(indoc! {r#"
        use cairo_lang_macro::{inline_macro, quote, ProcMacroResult, TokenStream};
        use cairo_lang_parser::utils::SimpleParserDatabase;
        use cairo_lang_syntax::node::with_db::SyntaxNodeWithDb;

        #[inline_macro]
        pub fn some(token_stream: TokenStream) -> ProcMacroResult {
            let db_val = SimpleParserDatabase::default();
            let db = &db_val;
            let (body, _diagnostics) = db.parse_token_stream_expr(&token_stream);
            let body = SyntaxNodeWithDb::new(&body, db);
            let result = ProcMacroResult::new(quote!{
                #body
            });
            result
        }
        "#})
        .build(&t);
    let project = temp.child("hello");
    ProjectBuilder::start()
        .name("hello")
        .version("1.0.0")
        .dep("some", &t)
        .lib_cairo(indoc! {r#"
            fn main() -> felt252 {
                let _x = some!(12);
                12
            }
        "#})
        .build(&project);

    Scarb::quick_snapbox()
        .arg("build")
        // Disable output from Cargo.
        .env("CARGO_TERM_QUIET", "true")
        .current_dir(&project)
        .assert()
        .success()
        .stdout_eq(indoc! {r#"
            [..]Compiling some v1.0.0 ([..]Scarb.toml)
            [..]Compiling hello v1.0.0 ([..]Scarb.toml)
            [..]Finished `dev` profile target(s) in [..]
        "#});
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

    Scarb::quick_snapbox()
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
fn only_compiles_needed_macros() {
    let t = TempDir::new().unwrap();
    let some = t.child("some");
    CairoPluginProjectBuilder::default()
        .name("some")
        .build(&some);
    let other = t.child("other");
    CairoPluginProjectBuilder::default()
        .name("other")
        .build(&other);
    let hello = t.child("hello");
    ProjectBuilder::start()
        .name("hello")
        .version("1.0.0")
        .dep("some", &some)
        .build(&hello);
    WorkspaceBuilder::start()
        .add_member("other")
        .add_member("some")
        .add_member("hello")
        .build(&t);
    Scarb::quick_snapbox()
        .arg("build")
        .args(vec!["-p", "hello"])
        // Disable output from Cargo.
        .env("CARGO_TERM_QUIET", "true")
        .current_dir(&t)
        .assert()
        .success()
        .stdout_eq(indoc! {r#"
            [..]Compiling some v1.0.0 ([..]Scarb.toml)
            [..]Compiling hello v1.0.0 ([..]Scarb.toml)
            [..]Finished `dev` profile target(s) in [..]
        "#});
}

#[test]
fn always_compile_macros_requested_with_package_filter() {
    let t = TempDir::new().unwrap();
    let some = t.child("some");
    CairoPluginProjectBuilder::default()
        .name("some")
        .build(&some);
    let other = t.child("other");
    CairoPluginProjectBuilder::default()
        .name("other")
        .build(&other);
    let hello = t.child("hello");
    ProjectBuilder::start()
        .name("hello")
        .version("1.0.0")
        .dep("some", &some)
        .build(&hello);
    WorkspaceBuilder::start()
        .add_member("other")
        .add_member("some")
        .add_member("hello")
        .build(&t);
    Scarb::quick_snapbox()
        .arg("build")
        .arg("--workspace")
        // Disable output from Cargo.
        .env("CARGO_TERM_QUIET", "true")
        .current_dir(&t)
        .assert()
        .success()
        .stdout_eq(indoc! {r#"
            [..]Compiling other v1.0.0 ([..]Scarb.toml)
            [..]Compiling some v1.0.0 ([..]Scarb.toml)
            [..]Compiling hello v1.0.0 ([..]Scarb.toml)
            [..]Finished `dev` profile target(s) in [..]
        "#});
}

#[test]
fn can_emit_diagnostic_with_custom_location() {
    let temp = TempDir::new().unwrap();
    let t = temp.child("some");
    CairoPluginProjectBuilder::default()
        .lib_rs(indoc! {r#"
        use cairo_lang_macro::{ProcMacroResult, TokenStream, TokenTree, attribute_macro, Diagnostic, TextSpan};

        #[attribute_macro]
        pub fn some(_attr: TokenStream, token_stream: TokenStream) -> ProcMacroResult {
            let mut start_span = None;
            let mut end_span = None;

            for token_tree in token_stream.tokens.iter() {
                let TokenTree::Ident(token) = token_tree;
                if token.content.as_ref().contains("(") {
                    start_span = Some(token.span.clone());
                }
                if token.content.as_ref().contains(")") {
                    end_span = Some(token.span.clone());
                }
            }
            let result = ProcMacroResult::new(token_stream);


            // Emit error diagnostic if tuple type is found
            if let (Some(start), Some(end)) = (start_span, end_span) {
                // Create a custom span from start to end
                let custom_span = TextSpan::new(start.start, end.end);

                let diag = Diagnostic::span_error(custom_span, "Unsupported tuple type");
                result.with_diagnostics(diag.into())
            } else {
                result
            }
        }
        "#})
        .build(&t);
    let project = temp.child("hello");
    ProjectBuilder::start()
        .name("hello")
        .version("1.0.0")
        .dep("some", &t)
        .lib_cairo(indoc! {r#"
            #[some]
            struct X {
                x: felt252,
                y: (u32, u64),
            }
        "#})
        .build(&project);

    Scarb::quick_snapbox()
        .arg("build")
        // Disable output from Cargo.
        .env("CARGO_TERM_QUIET", "true")
        .current_dir(&project)
        .assert()
        .failure()
        .stdout_eq(indoc! {r#"
            [..]Compiling some v1.0.0 ([..]Scarb.toml)
            [..]Compiling hello v1.0.0 ([..]Scarb.toml)
            error: Plugin diagnostic: Unsupported tuple type
             --> [..]lib.cairo:4:8
                y: (u32, u64),
                   ^^^^^^^^^^

            error: could not compile `hello` due to [..] previous error
        "#});
}

#[test]
fn inline_macro_can_emit_diagnostic_with_custom_location() {
    let temp = TempDir::new().unwrap();
    let t = temp.child("some");
    CairoPluginProjectBuilder::default()
        .lib_rs(indoc! {r#"
        use cairo_lang_macro::{ProcMacroResult, TokenStream, inline_macro, Diagnostic, TextSpan};

        #[inline_macro]
        pub fn some(token_stream: TokenStream) -> ProcMacroResult {
            let result = ProcMacroResult::new(token_stream);
            let custom_span = TextSpan::new(0, 8);
            let diag = Diagnostic::span_error(custom_span, "Error from inline.");
            result.with_diagnostics(diag.into())
        }
        "#})
        .build(&t);
    let project = temp.child("hello");
    ProjectBuilder::start()
        .name("hello")
        .version("1.0.0")
        .dep("some", &t)
        .lib_cairo(indoc! {r#"
            fn main() -> felt252 {
                let _x = some!("abcdefghi");
                12
            }
        "#})
        .build(&project);

    Scarb::quick_snapbox()
        .arg("build")
        // Disable output from Cargo.
        .env("CARGO_TERM_QUIET", "true")
        .current_dir(&project)
        .assert()
        .failure()
        .stdout_eq(indoc! {r#"
            [..] Compiling some v1.0.0 ([..]Scarb.toml)
            [..] Compiling hello v1.0.0 ([..]Scarb.toml)
            error: Plugin diagnostic: Error from inline.
             --> [..]lib.cairo:2:19
                let _x = some!("abcdefghi");
                              ^^^^^^^^

            error: could not compile `hello` due to [..] previous error
        "#});
}

#[test]
fn can_emit_diagnostic_with_custom_location_on_node_with_trivia() {
    let temp = TempDir::new().unwrap();
    let t = temp.child("some");
    CairoPluginProjectBuilder::default()
        .lib_rs(indoc! {r#"
        use cairo_lang_macro::{ProcMacroResult, TokenStream, TokenTree, attribute_macro, Diagnostic, TextSpan};

        #[attribute_macro]
        pub fn some(_attr: TokenStream, token_stream: TokenStream) -> ProcMacroResult {
            let mut start_span = None;
            let mut end_span = None;

            for token_tree in token_stream.tokens.iter() {
                let TokenTree::Ident(token) = token_tree;
                if token.content.as_ref().contains("(") {
                    start_span = Some(token.span.clone());
                }
                if token.content.as_ref().contains(")") {
                    end_span = Some(token.span.clone());
                }
            }
            let result = ProcMacroResult::new(token_stream);


            // Emit error diagnostic if tuple type is found
            if let (Some(start), Some(end)) = (start_span, end_span) {
                // Create a custom span from start to end
                let custom_span = TextSpan::new(start.start, end.end);

                let diag = Diagnostic::span_error(custom_span, "Unsupported tuple type");
                result.with_diagnostics(diag.into())
            } else {
                result
            }
        }
        "#})
        .build(&t);
    let project = temp.child("hello");
    ProjectBuilder::start()
        .name("hello")
        .version("1.0.0")
        .dep("some", &t)
        .lib_cairo(indoc! {r#"
            #[some]
            struct X {
                x: felt252,
                y:
                    // Some node trivia to move offsets a bit.
                    (u32, u64),
            }
        "#})
        .build(&project);

    Scarb::quick_snapbox()
        .arg("build")
        // Disable output from Cargo.
        .env("CARGO_TERM_QUIET", "true")
        .current_dir(&project)
        .assert()
        .failure()
        .stdout_eq(indoc! {r#"
            [..]Compiling some v1.0.0 ([..]Scarb.toml)
            [..]Compiling hello v1.0.0 ([..]Scarb.toml)
            error: Plugin diagnostic: Unsupported tuple type
             --> [..]lib.cairo:6:9
                    (u32, u64),
                    ^^^^^^^^^^

            error: could not compile `hello` due to [..] previous error
        "#});
}

#[test]
fn can_emit_diagnostic_with_inversed_span() {
    let temp = TempDir::new().unwrap();
    let t = temp.child("some");
    CairoPluginProjectBuilder::default()
        .lib_rs(indoc! {r#"
        use cairo_lang_macro::{ProcMacroResult, TokenStream, TokenTree, attribute_macro, Diagnostic, TextSpan};

        #[attribute_macro]
        pub fn some(_attr: TokenStream, token_stream: TokenStream) -> ProcMacroResult {
            let mut start_span = None;
            let mut end_span = None;

            for token_tree in token_stream.tokens.iter() {
                let TokenTree::Ident(token) = token_tree;
                if token.content.as_ref().contains("(") {
                    start_span = Some(token.span.clone());
                }
                if token.content.as_ref().contains(")") {
                    end_span = Some(token.span.clone());
                }
            }
            let result = ProcMacroResult::new(token_stream);


            // Emit error diagnostic if tuple type is found
            if let (Some(start), Some(end)) = (start_span, end_span) {
                // Create a custom span from start to end
                let custom_span = TextSpan::new(start.start, end.end);

                let diag = Diagnostic::span_error(custom_span, "Unsupported tuple type");
                result.with_diagnostics(diag.into())
            } else {
                result
            }
        }
        "#})
        .build(&t);
    let project = temp.child("hello");
    ProjectBuilder::start()
        .name("hello")
        .version("1.0.0")
        .dep("some", &t)
        .lib_cairo(indoc! {r#"
            #[some]
            struct X {
                x: felt252,
                y: (u32, u64),
            }
        "#})
        .build(&project);

    Scarb::quick_snapbox()
        .arg("build")
        // Disable output from Cargo.
        .env("CARGO_TERM_QUIET", "true")
        .current_dir(&project)
        .assert()
        .failure()
        .stdout_eq(indoc! {r#"
            [..]Compiling some v1.0.0 ([..]Scarb.toml)
            [..]Compiling hello v1.0.0 ([..]Scarb.toml)
            error: Plugin diagnostic: Unsupported tuple type
             --> [..]lib.cairo:4:8
                y: (u32, u64),
                   ^^^^^^^^^^

            error: could not compile `hello` due to [..] previous error
        "#});
}

#[test]
fn can_emit_diagnostic_with_out_of_bounds_span() {
    let temp = TempDir::new().unwrap();
    let t = temp.child("some");
    CairoPluginProjectBuilder::default()
        .lib_rs(indoc! {r#"
        use cairo_lang_macro::{ProcMacroResult, TokenStream, attribute_macro, Diagnostic, TextSpan};

        #[attribute_macro]
        pub fn some(_attr: TokenStream, token_stream: TokenStream) -> ProcMacroResult {
            let diag = Diagnostic::span_warning(TextSpan::new(0, 1000000), "Hello world!");
            ProcMacroResult::new(token_stream).with_diagnostics(diag.into())
        }
        "#})
        .build(&t);
    let project = temp.child("hello");
    ProjectBuilder::start()
        .name("hello")
        .version("1.0.0")
        .dep("some", &t)
        .lib_cairo(indoc! {r#"
            #[some]
            fn main() -> felt252 { 12 }
        "#})
        .build(&project);

    Scarb::quick_snapbox()
        .arg("build")
        .env("CARGO_TERM_QUIET", "true")
        .current_dir(&project)
        .assert()
        .success()
        .stdout_eq(indoc! {r#"
            [..]Compiling some v1.0.0 ([..]Scarb.toml)
            [..]Compiling hello v1.0.0 ([..]Scarb.toml)
            warn: Plugin diagnostic: Hello world!
             --> [..]lib.cairo:1:1
            #[some]
            ^^^^^^^

            [..]Finished `dev` profile target(s) in [..]
        "#});
}

#[test]
fn can_emit_diagnostic_with_custom_location_with_parser() {
    let temp = TempDir::new().unwrap();
    let t = temp.child("some");
    CairoPluginProjectBuilder::default()
        .add_cairo_lang_parser_dep().add_cairo_lang_syntax_dep()
        .add_primitive_token_dep()
        .lib_rs(indoc! {r#"
        use cairo_lang_macro::{attribute_macro, quote, ProcMacroResult, TokenStream, TokenTree, TextSpan, Diagnostic};
        use cairo_lang_parser::utils::SimpleParserDatabase;
        use cairo_lang_syntax::node::with_db::SyntaxNodeWithDb;

        #[attribute_macro]
        pub fn some(_attr: TokenStream, token_stream: TokenStream) -> ProcMacroResult {
            let first_attr_start_span = token_stream
                .tokens
                .first()
                .map(|tt| match tt {
                    TokenTree::Ident(token) => token.span.clone(),
                })
                .unwrap();
            let first_attr_end_span = token_stream
                .tokens
                .iter()
                .find(|tt| match tt {
                    TokenTree::Ident(token) => token.content.as_ref() == "]",
                })
                .map(|tt| match tt {
                    TokenTree::Ident(token) => token.span.clone(),
                })
                .unwrap();

            let mut start_span = None;
            let mut end_span = None;

            for token_tree in token_stream.tokens.iter() {
                let TokenTree::Ident(token) = token_tree;
                if token.content.as_ref().contains("(") {
                    start_span = Some(token.span.clone());
                }
                if token.content.as_ref().contains(")") {
                    end_span = Some(token.span.clone());
                }
            }
            let db_val = SimpleParserDatabase::default();
            let db = &db_val;
            let (body, _diagnostics) = db.parse_token_stream(&token_stream);
            let body = SyntaxNodeWithDb::new(&body, db);
            let result = ProcMacroResult::new(quote!{
                fn other() {}

                #body
            });

            // Emit error diagnostic if tuple type is found
            if let (Some(start), Some(end)) = (start_span, end_span) {
                // Create a custom span from start to end
                let custom_span = TextSpan::new(start.start, end.end);
                let diag1 = Diagnostic::span_error(custom_span, "Unsupported tuple type");

                let custom_span = TextSpan::new(first_attr_start_span.start, first_attr_end_span.end);
                let diag2 = Diagnostic::span_warning(
                    custom_span,
                    "This is a warning from the macro.",
                );

                result.with_diagnostics(vec![diag1, diag2].into())
            } else {
                result
            }
        }
        "#})
        .build(&t);
    let project = temp.child("hello");
    ProjectBuilder::start()
        .name("hello")
        .version("1.0.0")
        .dep("some", &t)
        .lib_cairo(indoc! {r#"
            #[doc(hidden)]
            #[some]
            struct X {
                x: felt252,
                y: (u32, u64),
            }
        "#})
        .build(&project);

    Scarb::quick_snapbox()
        .arg("build")
        // Disable output from Cargo.
        .env("CARGO_TERM_QUIET", "true")
        .current_dir(&project)
        .assert()
        .failure()
        .stdout_eq(indoc! {r#"
            [..]Compiling some v1.0.0 ([..]Scarb.toml)
            [..]Compiling hello v1.0.0 ([..]Scarb.toml)
            error: Plugin diagnostic: Unsupported tuple type
             --> [..]lib.cairo:5:8
                y: (u32, u64),
                   ^^^^^^^^^^

            warn: Plugin diagnostic: This is a warning from the macro.
             --> [..]lib.cairo:1:1
            #[doc(hidden)]
            ^^^^^^^^^^^^^^

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

    Scarb::quick_snapbox()
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
fn attribute_diags_mapped_correctly_to_call_site() {
    let temp = TempDir::new().unwrap();
    let t = temp.child("some");
    CairoPluginProjectBuilder::default()
        .add_primitive_token_dep()
        .lib_rs(indoc! {r#"
            use cairo_lang_macro::{attribute_macro, quote, ProcMacroResult, TokenStream};

            #[attribute_macro]
            pub fn improper_attribute_macro_v2(_args: TokenStream, item: TokenStream) -> ProcMacroResult {
                let ts = quote! {
                    #item

                    fn added_fun_v2() {
                        a = b;
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
            #[improper_attribute_macro_v2]
            fn foo() {
            }
        "#})
        .build(&project);

    Scarb::quick_snapbox()
        .arg("build")
        // Disable output from Cargo.
        .env("CARGO_TERM_QUIET", "true")
        .current_dir(&project)
        .assert()
        .failure()
        .stdout_eq(indoc! {r#"
            [..] Compiling some v1.0.0 ([..]Scarb.toml)
            [..] Compiling hello v1.0.0 ([..]Scarb.toml)
            error[E0006]: Identifier not found.
             --> [..]lib.cairo:1:1
            #[improper_attribute_macro_v2]
            ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
            note: this error originates in the attribute macro: `improper_attribute_macro_v2`
            
            error: Invalid left-hand side of assignment.
             --> [..]lib.cairo:1:1
            #[improper_attribute_macro_v2]
            ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
            note: this error originates in the attribute macro: `improper_attribute_macro_v2`
            
            error: could not compile `hello` due to [..] previous error[..]
       "#});
}

#[test]
fn inline_macro_diags_mapped_correctly_to_call_site() {
    let temp = TempDir::new().unwrap();
    let t = temp.child("some");
    CairoPluginProjectBuilder::default()
        .add_primitive_token_dep()
        .lib_rs(indoc! {r#"
            use cairo_lang_macro::{inline_macro, quote, ProcMacroResult, TokenStream};

            #[inline_macro]
            pub fn improper_inline_macro_v2(item: TokenStream) -> ProcMacroResult {
                let ts = quote! {
                    {
                        #item;
                        unbound_identifier_v2
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
            fn foo() {
                improper_inline_macro_v2!(10 + 10);
            }
        "#})
        .build(&project);

    Scarb::quick_snapbox()
        .arg("build")
        // Disable output from Cargo.
        .env("CARGO_TERM_QUIET", "true")
        .current_dir(&project)
        .assert()
        .failure()
        .stdout_eq(indoc! {r#"
            [..] Compiling some v1.0.0 ([..]Scarb.toml)
            [..] Compiling hello v1.0.0 ([..]Scarb.toml)
            error[E0006]: Identifier not found.
             --> [..]lib.cairo:2:5
                improper_inline_macro_v2!(10 + 10);
                ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

            error: could not compile `hello` due to [..] previous error
       "#});
}

#[test]
fn call_site_mapped_correctly_after_expansion_by_two_macros() {
    let temp = TempDir::new().unwrap();
    let t = temp.child("some");
    CairoPluginProjectBuilder::default()
        .add_primitive_token_dep()
        .lib_rs(indoc! {r#"
            use cairo_lang_macro::{attribute_macro, quote, ProcMacroResult, TokenStream};

            #[attribute_macro]
            pub fn simple_attribute_macro_v2(_args: TokenStream, item: TokenStream) -> ProcMacroResult {
                let ts = quote! {
                    #item

                    fn generated_function_v2() {}
                    fn generated_function_v2() {}
                };
                ProcMacroResult::new(ts)
            }

            #[attribute_macro]
            pub fn complex_attribute_macro_v2(_args: TokenStream, item: TokenStream) -> ProcMacroResult {
                let ts = quote! {
                    #item

                    #[simple_attribute_macro_v2]
                    fn generated_function_with_other_attribute_v2() {}
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
            #[complex_attribute_macro_v2]
            fn foo() {}
        "#})
        .build(&project);

    Scarb::quick_snapbox()
        .arg("build")
        // Disable output from Cargo.
        .env("CARGO_TERM_QUIET", "true")
        .current_dir(&project)
        .assert()
        .failure()
        .stdout_eq(indoc! {r#"
            [..] Compiling some v1.0.0 ([..]Scarb.toml)
            [..] Compiling hello v1.0.0 ([..]Scarb.toml)
            error: The name `generated_function_v2` is defined multiple times.
             --> [..]lib.cairo:1:1
            #[complex_attribute_macro_v2]
            ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
            note: this error originates in the attribute macro: `complex_attribute_macro_v2`
            note: this error originates in the attribute macro: `simple_attribute_macro_v2`

            error: could not compile `hello` due to [..] previous error
       "#});
}

#[test]
fn span_offsets_calculated_correctly_for_function_with_non_macro_attrs() {
    let temp = TempDir::new().unwrap();
    let t = temp.child("some");
    CairoPluginProjectBuilder::default()
        .add_primitive_token_dep()
        .add_cairo_lang_parser_dep()
        .add_cairo_lang_syntax_dep()
        .lib_rs(indoc! {r#"
            use cairo_lang_macro::{attribute_macro, quote, ProcMacroResult, TokenStream};
            use cairo_lang_parser::utils::SimpleParserDatabase;
            use cairo_lang_syntax::node::with_db::SyntaxNodeWithDb;

            #[attribute_macro]
            pub fn simple_attr(_args: TokenStream, item: TokenStream) -> ProcMacroResult {
                let db_val = SimpleParserDatabase::default();
                let db = &db_val;
                let (body, _diagnostics) = db.parse_token_stream(&item);
                let body = SyntaxNodeWithDb::new(&body, db);
                // Note we generate a new function here.
                // We only do this to ensure, that the resulting code differs from the original one.
                // Otherwise, as an optimization, Scarb won't rewrite the AST node.
                let ts = quote! {
                    fn other() {}

                    #body
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
        .dep_builtin("assert_macros")
        .lib_cairo(indoc! {r#"
            #[doc(hidden)]
            #[simple_attr]
            fn foo() {
                assert(1 + 1 == 2, 'fail')
                let _a = 1;
            }
        "#})
        .build(&project);

    Scarb::quick_snapbox()
        .arg("build")
        // Disable output from Cargo.
        .env("CARGO_TERM_QUIET", "true")
        .current_dir(&project)
        .assert()
        .failure()
        .stdout_eq(indoc! {r#"
            [..] Compiling some v1.0.0 ([..]Scarb.toml)
            [..] Compiling hello v1.0.0 ([..]Scarb.toml)
            error: Missing semicolon
             --> [..]lib.cairo:4:31
                assert(1 + 1 == 2, 'fail')
                                          ^
            note: this error originates in the attribute macro: `simple_attr`

            error: could not compile `hello` due to [..] previous error
       "#});
}

#[test]
fn token_stream_parsed_with_correct_spans() {
    let temp = TempDir::new().unwrap();
    let t = temp.child("some");
    CairoPluginProjectBuilder::default()
        .add_primitive_token_dep()
        .add_cairo_lang_parser_dep()
        .add_cairo_lang_syntax_dep()
        .lib_rs(indoc! {r#"
            use cairo_lang_macro::{attribute_macro, quote, ProcMacroResult, TokenStream};
            use cairo_lang_parser::utils::SimpleParserDatabase;
            use cairo_lang_syntax::node::with_db::SyntaxNodeWithDb;

            #[attribute_macro]
            pub fn simple_attr(_args: TokenStream, item: TokenStream) -> ProcMacroResult {
                let db_val = SimpleParserDatabase::default();
                let db = &db_val;
                let (body, _diagnostics) = db.parse_token_stream(&item);
                let body = SyntaxNodeWithDb::new(&body, db);
                let ts = quote! {
                    #body
                };
                for token in &ts.tokens {
                    println!("{:?}", &token);
                }
                ProcMacroResult::new(ts)
            }

        "#})
        .build(&t);
    let project = temp.child("hello");
    ProjectBuilder::start()
        .name("hello")
        .version("1.0.0")
        .dep("some", &t)
        .dep_builtin("assert_macros")
        // Note we add leading whitespace before function declaration.
        // This cannot affect the span in resulting token stream.
        .lib_cairo(indoc! {r#"
            #[doc(hidden)]
            fn other() {}


            #[simple_attr]
              fn foo() {
                let _a = 1;
            }
        "#})
        .build(&project);

    Scarb::quick_snapbox()
        .arg("build")
        // Disable output from Cargo.
        .env("CARGO_TERM_QUIET", "true")
        .current_dir(&project)
        .assert()
        .success()
        .stdout_eq(indoc! {r##"
            [..] Compiling some v1.0.0 ([..]Scarb.toml)
            [..] Compiling hello v1.0.0 ([..]Scarb.toml)
            Ident(Token { content: "  ", span: TextSpan { start: 0, end: 2 } })
            Ident(Token { content: "fn", span: TextSpan { start: 2, end: 4 } })
            Ident(Token { content: " ", span: TextSpan { start: 4, end: 5 } })
            Ident(Token { content: "foo", span: TextSpan { start: 5, end: 8 } })
            Ident(Token { content: "(", span: TextSpan { start: 8, end: 9 } })
            Ident(Token { content: ")", span: TextSpan { start: 9, end: 10 } })
            Ident(Token { content: " ", span: TextSpan { start: 10, end: 11 } })
            Ident(Token { content: "{", span: TextSpan { start: 11, end: 12 } })
            Ident(Token { content: "
            ", span: TextSpan { start: 12, end: 13 } })
            Ident(Token { content: "    ", span: TextSpan { start: 13, end: 17 } })
            Ident(Token { content: "let", span: TextSpan { start: 17, end: 20 } })
            Ident(Token { content: " ", span: TextSpan { start: 20, end: 21 } })
            Ident(Token { content: "_a", span: TextSpan { start: 21, end: 23 } })
            Ident(Token { content: " ", span: TextSpan { start: 23, end: 24 } })
            Ident(Token { content: "=", span: TextSpan { start: 24, end: 25 } })
            Ident(Token { content: " ", span: TextSpan { start: 25, end: 26 } })
            Ident(Token { content: "1", span: TextSpan { start: 26, end: 27 } })
            Ident(Token { content: ";", span: TextSpan { start: 27, end: 28 } })
            Ident(Token { content: "
            ", span: TextSpan { start: 28, end: 29 } })
            Ident(Token { content: "}", span: TextSpan { start: 29, end: 30 } })
            Ident(Token { content: "
            ", span: TextSpan { start: 30, end: 31 } })
            [..]Finished `dev` profile target(s) in [..]
      "##});
}

#[test]
fn zero_width_diags_mapped_correctly_at_token_starts() {
    let temp = TempDir::new().unwrap();
    let t = temp.child("some");
    CairoPluginProjectBuilder::default()
        .add_primitive_token_dep()
        .add_cairo_lang_parser_dep()
        .add_cairo_lang_syntax_dep()
        .lib_rs(indoc! {r#"
            use cairo_lang_macro::{attribute_macro, quote, ProcMacroResult, TokenStream};
            use cairo_lang_parser::utils::SimpleParserDatabase;
            use cairo_lang_syntax::node::with_db::SyntaxNodeWithDb;

            fn parse_item(item: TokenStream) -> TokenStream {
                let db_val = SimpleParserDatabase::default();
                let db = &db_val;
                let (body, _diagnostics) = db.parse_token_stream(&item);
                let body = SyntaxNodeWithDb::new(&body, db);
                quote!(#body)
            }

            #[attribute_macro]
            pub fn first_attr(_args: TokenStream, item: TokenStream) -> ProcMacroResult {
                let body = parse_item(item);
                // Note we generate a new function here.
                // We only do this to ensure, that the resulting code differs from the original one.
                // Otherwise, as an optimization, Scarb won't rewrite the AST node.
                let ts = quote! {
                    fn other_1() {}

                    #body
                };
                ProcMacroResult::new(ts)
            }
          
            #[attribute_macro]
            pub fn second_attr(_args: TokenStream, item: TokenStream) -> ProcMacroResult {
                let body = parse_item(item);
                // Note we generate a new function here.
                // We only do this to ensure, that the resulting code differs from the original one.
                // Otherwise, as an optimization, Scarb won't rewrite the AST node.
                let ts = quote! {
                    fn other_2() {}

                    #body
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
        .dep_builtin("assert_macros")
        .lib_cairo(indoc! {r#"
            #[first_attr]
            #[doc(hidden)]
            #[second_attr]
            fn foo() {
                let y = x;
            }
        "#})
        .build(&project);

    Scarb::quick_snapbox()
        .arg("build")
        // Disable output from Cargo.
        .env("CARGO_TERM_QUIET", "true")
        .current_dir(&project)
        .assert()
        .failure()
        .stdout_eq(indoc! {r#"
            [..] Compiling some v1.0.0 ([..]Scarb.toml)
            [..] Compiling hello v1.0.0 ([..]Scarb.toml)
            error[E0006]: Identifier not found.
             --> [..]lib.cairo:5:13
                let y = x;
                        ^
            note: this error originates in the attribute macro: `first_attr`
            note: this error originates in the attribute macro: `second_attr`
            
            warn[E0001]: Unused variable. Consider ignoring by prefixing with `_`.
             --> [..]lib.cairo:5:9
                let y = x;
                    ^
            note: this error originates in the attribute macro: `first_attr`
            note: this error originates in the attribute macro: `second_attr`
            
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

    Scarb::quick_snapbox()
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
fn module_level_inline_macro() {
    let temp = TempDir::new().unwrap();
    let t = temp.child("some");

    CairoPluginProjectBuilder::default()
        .lib_rs(indoc! {r##"
        use cairo_lang_macro::{ProcMacroResult, TokenStream, inline_macro, TokenTree, Token, TextSpan};

        #[inline_macro]
        pub fn some(_token_stream: TokenStream) -> ProcMacroResult {
            let code = indoc::formatdoc!{r#"
                pub fn foo() -> felt252 {{ 42 }}
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
            mod hello {
                some!();
            }

            #[executable]
            fn main() -> felt252 {
                hello::foo()
            }
        "#})
        .build(&project);

    Scarb::quick_snapbox()
        .arg("execute")
        .arg("--print-program-output")
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
            42
            Saving output to: target/execute/hello/execution1
        "#});
}

#[test]
fn module_level_inline_macro_with_args() {
    let temp = TempDir::new().unwrap();
    let t = temp.child("some");
    CairoPluginProjectBuilder::default()
            .lib_rs(indoc! {r##"
            use cairo_lang_macro::{ProcMacroResult, TokenStream, inline_macro, TokenTree, Token, TextSpan};

            #[inline_macro]
            pub fn some(token_stream: TokenStream) -> ProcMacroResult {
                // Parse single value from token stream
                let input = token_stream.to_string().trim().to_string();

                let code = indoc::formatdoc!{r#"
                    pub fn foo() -> felt252 {{
                        {input}
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
            mod hello {
                some!(100);
            }

            #[executable]
            fn main() -> felt252 {
                hello::foo()
            }
        "#})
        .build(&project);

    Scarb::quick_snapbox()
        .arg("execute")
        .arg("--print-program-output")
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
            100
            Saving output to: target/execute/hello/execution1
        "#});
}

#[test]
fn module_level_inline_macro_in_nested_module() {
    let temp = TempDir::new().unwrap();
    let t = temp.child("some");
    CairoPluginProjectBuilder::default()
        .lib_rs(indoc! {r##"
        use cairo_lang_macro::{ProcMacroResult, TokenStream, inline_macro, TokenTree, Token, TextSpan};

        #[inline_macro]
        pub fn some(_token_stream: TokenStream) -> ProcMacroResult {
            let code = indoc::formatdoc!{r#"
                pub fn nested_foo() -> felt252 {{ 42 }}
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
            mod outer {
                pub mod inner {
                    some!();
                }
            }

            #[executable]
            fn main() -> felt252 {
                outer::inner::nested_foo()
            }
        "#})
        .build(&project);

    Scarb::quick_snapbox()
        .arg("execute")
        .arg("--print-program-output")
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
            42
            Saving output to: target/execute/hello/execution1
        "#});
}

#[test]
fn module_level_inline_macro_empty() {
    let temp = TempDir::new().unwrap();
    let t = temp.child("some");
    CairoPluginProjectBuilder::default()
        .lib_rs(indoc! {r##"
        use cairo_lang_macro::{ProcMacroResult, TokenStream, inline_macro};

        #[inline_macro]
        pub fn empty_foo(_token_stream: TokenStream) -> ProcMacroResult {
            ProcMacroResult::new(TokenStream::empty())
        }
        "##})
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
            mod hello {
                empty_foo!();
                pub fn a() -> felt252 { 21 }
                empty_foo!();
                pub fn b() -> felt252 { 42 }
                empty_foo!();
            }

            #[executable]
            fn main() -> felt252 {
                hello::a() + hello::b()
            }
        "#})
        .build(&project);

    Scarb::quick_snapbox()
        .arg("execute")
        .arg("--print-program-output")
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
            63
            Saving output to: target/execute/hello/execution1
        "#});
}

#[test]
fn module_level_inline_macro_can_emit_diagnostics() {
    let temp = TempDir::new().unwrap();
    let t = temp.child("some");
    CairoPluginProjectBuilder::default()
        .lib_rs(indoc! {r##"
        use cairo_lang_macro::{ProcMacroResult, TokenStream, inline_macro, Diagnostic};

        #[inline_macro]
        pub fn some(_token_stream: TokenStream) -> ProcMacroResult {
            let diag = Diagnostic::error("Some error from macro.");
            ProcMacroResult::new(TokenStream::empty())
                .with_diagnostics(diag.into())
        }
        "##})
        .build(&t);
    let project = temp.child("hello");
    ProjectBuilder::start()
        .name("hello")
        .version("1.0.0")
        .dep("some", &t)
        .lib_cairo(indoc! {r#"
            mod hello {
                some!();
            }
        "#})
        .build(&project);

    Scarb::quick_snapbox()
        .arg("build")
        .env("CARGO_TERM_QUIET", "true")
        .current_dir(&project)
        .assert()
        .failure()
        .stdout_eq(indoc! {r#"
            [..] Compiling some v1.0.0 ([..]Scarb.toml)
            [..] Compiling hello v1.0.0 ([..]Scarb.toml)
            error: Plugin diagnostic: Some error from macro.
             --> [..]lib.cairo:2:5
                some!();
                ^^^^^^^^

            error: could not compile `hello` due to 1 previous error
        "#});
}

#[test]
fn module_level_inline_macro_code_mappings_preserve_attribute_error_locations() {
    let temp = TempDir::new().unwrap();
    let t = temp.child("some");
    CairoPluginProjectBuilder::default()
        .lib_rs(indoc! {r##"
        use cairo_lang_macro::{inline_macro, ProcMacroResult, TokenStream, TokenTree, Token, TextSpan};

        #[inline_macro]
        pub fn some(token_stream: TokenStream) -> ProcMacroResult {
            let mut tokens = Vec::new();

            tokens.push(TokenTree::Ident(Token::new(
                "fn inner() -> felt252 { ".to_string(),
                TextSpan::call_site(),
            )));

            // The first token is the opening parenthesis "(", so take the next token
            let TokenTree::Ident(arg) = token_stream
                .tokens
                .get(1)
                .expect("macro expected at least one argument");

            let content = arg.content.as_ref();
            let slice_len: usize = core::cmp::min(7, content.len());
            let slice = &content[..slice_len];

            tokens.push(TokenTree::Ident(Token::new(
                slice.to_string(),
                TextSpan::new(2, 7),
            )));

            tokens.push(TokenTree::Ident(Token::new(
                " }".to_string(),
                TextSpan::call_site(),
            )));
            ProcMacroResult::new(TokenStream::new(tokens))
        }
        "##})
        .build(&t);

    let project = temp.child("hello");
    ProjectBuilder::start()
        .name("hello")
        .version("1.0.0")
        .dep("some", &t)
        .lib_cairo(indoc! {r#"
            mod my_module {
                some!(abcdefghi);
            }


        "#})
        .build(&project);

    Scarb::quick_snapbox()
        .arg("build")
        .env("CARGO_TERM_QUIET", "true")
        .current_dir(&project)
        .assert()
        .failure()
        .stdout_eq(indoc! {r#"
            [..] Compiling some v1.0.0 ([..]Scarb.toml)
            [..] Compiling hello v1.0.0 ([..]Scarb.toml)
            error[E0006]: Identifier not found.
             --> [..]lib.cairo:2:12
                some!(abcdefghi);
                       ^^^^^
            note: this error originates in the inline macro: `some`

            error: could not compile `hello` due to 1 previous error
        "#});
}

#[test]
fn module_level_inline_macro_multiple() {
    let temp = TempDir::new().unwrap();
    let t = temp.child("some");
    CairoPluginProjectBuilder::default()
        .lib_rs(indoc! {r##"
        use cairo_lang_macro::{ProcMacroResult, TokenStream, inline_macro, TokenTree, Token, TextSpan};

        #[inline_macro]
        pub fn some(_token_stream: TokenStream) -> ProcMacroResult {
            let code = indoc::formatdoc!{r#"
                pub fn foo() -> felt252 {{ 21 }}
            "#};
            ProcMacroResult::new(TokenStream::new(vec![TokenTree::Ident(Token::new(
                code.clone(),
                TextSpan {
                    start: 0,
                    end: code.len() as u32,
                },
            ))]))
        }

        #[inline_macro]
        pub fn other(_token_stream: TokenStream) -> ProcMacroResult {
            let code = indoc::formatdoc!{r#"
                pub fn bar() -> felt252 {{ 42 }}
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
            mod hello {
                some!();
                other!();
            }

            #[executable]
            fn main() -> felt252 {
                hello::foo() + hello::bar()
            }
        "#})
        .build(&project);

    Scarb::quick_snapbox()
        .arg("execute")
        .arg("--print-program-output")
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
            63
            Saving output to: target/execute/hello/execution1
        "#});
}
