use assert_fs::TempDir;
use assert_fs::fixture::PathChild;
use indoc::indoc;
use scarb_test_support::cairo_plugin_project_builder::CairoPluginProjectBuilder;
use scarb_test_support::command::Scarb;
use scarb_test_support::project_builder::ProjectBuilder;

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
            error: Inline macro `some` not found.
             --> [..]lib.cairo:2:14
                let _x = some!();
                         ^^^^^^^

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

    Scarb::quick_command()
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

    Scarb::quick_command()
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

    Scarb::quick_command()
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
            error: Plugin diagnostic: Error from inline.
             --> [..]lib.cairo:2:19
                let _x = some!("abcdefghi");
                              ^^^^^^^^

            error: could not compile `hello` due to [..] previous error
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
            error[E0006]: Identifier not found.
             --> [..]lib.cairo:2:5
                improper_inline_macro_v2!(10 + 10);
                ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

            error: could not compile `hello` due to [..] previous error
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

    Scarb::quick_command()
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
        .add_cairo_lang_parser_dep()
        .add_cairo_lang_syntax_dep()
        .add_primitive_token_dep()
        .lib_rs(indoc! {r##"
            use cairo_lang_macro::{ProcMacroResult, TokenStream, inline_macro, quote};
            use cairo_lang_parser::utils::SimpleParserDatabase;
            use cairo_lang_syntax::node::with_db::SyntaxNodeWithDb;

            #[inline_macro]
            pub fn some(token_stream: TokenStream) -> ProcMacroResult {
                let db_val = SimpleParserDatabase::default();
                let db = &db_val;
                let (body, _diagnostics) = db.parse_token_stream_expr(&token_stream);
                let body = SyntaxNodeWithDb::new(&body, db);
                let result = ProcMacroResult::new(quote!{
                    pub fn foo() -> felt252 {
                        #body
                    }
                });
                result
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

    Scarb::quick_command()
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
fn module_level_inline_macro_module_tree_root() {
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
            some!();

            #[executable]
            fn main() -> felt252 {
                foo()
            }
        "#})
        .build(&project);

    Scarb::quick_command()
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

    Scarb::quick_command()
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

    Scarb::quick_command()
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
fn module_level_inline_macro_code_mappings_preserve_error_locations() {
    let temp = TempDir::new().unwrap();
    let t = temp.child("some");
    CairoPluginProjectBuilder::default()
        .lib_rs(indoc! {r##"
        use cairo_lang_macro::{inline_macro, ProcMacroResult, TokenStream, TokenTree, Token, TextSpan};

        #[inline_macro]
        pub fn some(_token_stream: TokenStream) -> ProcMacroResult {
            let mut tokens = Vec::new();

            tokens.push(TokenTree::Ident(Token::new(
                "fn inner() -> felt252 { ".to_string(),
                TextSpan::call_site(),
            )));

            tokens.push(TokenTree::Ident(Token::new(
                "undef".to_string(),
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
            mod hello {
                some!(abcdefghi);
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

    Scarb::quick_command()
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
