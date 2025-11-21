use assert_fs::TempDir;
use assert_fs::fixture::PathChild;
use indoc::indoc;
use scarb_test_support::cairo_plugin_project_builder::CairoPluginProjectBuilder;
use scarb_test_support::command::Scarb;
use scarb_test_support::fsx::ChildPathEx;
use scarb_test_support::project_builder::ProjectBuilder;
use snapbox::Assert;

#[test]
fn quote_macro() {
    let temp = TempDir::new().unwrap();
    let t = temp.child("some");
    CairoPluginProjectBuilder::default()
        .add_primitive_token_dep()
        .lib_rs(indoc! {r##"
            use cairo_lang_macro::{ProcMacroResult, TokenStream, inline_macro, quote};
            #[inline_macro]
            pub fn some(_token_stream: TokenStream) -> ProcMacroResult {
                let tokens = quote! {
                    5
                };
                ProcMacroResult::new(tokens)
            }
        "##})
        .build(&t);
    let project = temp.child("hello");
    ProjectBuilder::start()
        .name("hello")
        .version("1.0.0")
        .dep_cairo_execute()
        .manifest_extra(indoc! {r#"
            [executable]
            [cairo]
            enable-gas = false
        "#})
        .dep("some", &t)
        .lib_cairo(indoc! {r#"
            #[executable]
            fn main() -> felt252 { some!() }
        "#})
        .build(&project);

    Scarb::quick_command()
        .arg("execute")
        .arg("--print-program-output")
        // Disable output from Cargo.
        .env("CARGO_TERM_QUIET", "true")
        .current_dir(&project)
        .assert()
        .stdout_eq(indoc! {r#"
        [..] Compiling some v1.0.0 [..]
        [..] Compiling hello v1.0.0 [..]
        [..] Finished `dev` profile [..]
        [..]Executing hello
        Program output:
        5
        Saving output to: target/execute/hello/execution1
        "#})
        .success();
}

#[test]
fn quote_macro_with_token_tree() {
    let temp = TempDir::new().unwrap();
    let t = temp.child("some");
    CairoPluginProjectBuilder::default()
        .add_primitive_token_dep()
        .lib_rs(indoc! {r##"
        use cairo_lang_macro::{ProcMacroResult, TokenStream, inline_macro, TokenTree, Token, TextSpan, quote};
        #[inline_macro]
        pub fn some(_token_stream: TokenStream) -> ProcMacroResult {
          let token = TokenTree::Ident(Token::new("5".to_string(), TextSpan::call_site()));
          let tokens = quote! {
            #token
          };
          ProcMacroResult::new(tokens)
        }
        "##})
        .build(&t);
    let project = temp.child("hello");
    ProjectBuilder::start()
        .name("hello")
        .version("1.0.0")
        .dep_cairo_execute()
        .manifest_extra(indoc! {r#"
            [executable]
            [cairo]
            enable-gas = false
        "#})
        .dep("some", &t)
        .lib_cairo(indoc! {r#"
            #[executable]
            fn main() -> felt252 {
              some!()
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
        .stdout_eq(indoc! {r#"
        [..] Compiling some v1.0.0 [..]
        [..] Compiling hello v1.0.0 [..]
        [..] Finished `dev` profile [..]
        [..]Executing hello
        Program output:
        5
        Saving output to: target/execute/hello/execution1
        "#})
        .success();
}

#[test]
fn quote_macro_with_token_stream() {
    let temp = TempDir::new().unwrap();
    let t = temp.child("some");
    CairoPluginProjectBuilder::default()
        .add_primitive_token_dep()
        .lib_rs(indoc! {r##"
        use cairo_lang_macro::{ProcMacroResult, TokenStream, inline_macro, TokenTree, Token, TextSpan, quote};
        #[inline_macro]
        pub fn some(_token_stream: TokenStream) -> ProcMacroResult {
          let token = TokenStream::new(vec![TokenTree::Ident(Token::new("5".to_string(), TextSpan::call_site()))]);
          let tokens = quote! {
            #token
          };
          ProcMacroResult::new(tokens)
        }
      "##})
        .build(&t);
    let project = temp.child("hello");
    ProjectBuilder::start()
        .name("hello")
        .version("1.0.0")
        .dep_cairo_execute()
        .manifest_extra(indoc! {r#"
            [executable]
            [cairo]
            enable-gas = false
        "#})
        .dep("some", &t)
        .lib_cairo(indoc! {r#"
            #[executable]
            fn main() -> felt252 {
              some!()
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
        .stdout_eq(indoc! {r#"
               Compiling some v1.0.0 ([..]Scarb.toml)
               Compiling hello v1.0.0 ([..]Scarb.toml)
                Finished `dev` profile target(s) in [..]
            [..]Executing hello
            Program output:
            5
            Saving output to: target/execute/hello/execution1
        "#})
        .success();
}

#[test]
fn quote_macro_with_syntax_node() {
    let temp = TempDir::new().unwrap();
    let t = temp.child("some");
    CairoPluginProjectBuilder::default()
        .add_primitive_token_dep()
        .add_cairo_lang_parser_dep()
        .add_cairo_lang_syntax_dep()
        .lib_rs(indoc! {r##"
        use cairo_lang_macro::{ProcMacroResult, TokenStream, attribute_macro, quote};
        use cairo_lang_parser::utils::SimpleParserDatabase;
        use cairo_lang_syntax::node::with_db::SyntaxNodeWithDb;
        #[attribute_macro]
        pub fn some(_attr: TokenStream, token_stream: TokenStream) -> ProcMacroResult {
          let db_val = SimpleParserDatabase::default();
          let db = &db_val;
          let code = r#"
              fn main() -> felt252 {
                5
              }
          "#;
          let syntax_node = db.parse_virtual(code).unwrap();
          let syntax_node_with_db = SyntaxNodeWithDb::new(&syntax_node, db);
          let tokens = quote! {
            #syntax_node_with_db
          };
          ProcMacroResult::new(tokens)
        }
      "##})
        .build(&t);
    let project = temp.child("hello");
    ProjectBuilder::start()
        .name("hello")
        .version("1.0.0")
        .dep("some", &t)
        .lib_cairo(indoc! {r#"
            #[some]
            fn main() -> u32 {
              // completly wrong type
              true
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
                fn main() -> felt252 {
                    5
                }
            }
        "#},
    );
}

#[test]
fn quote_format_macro_supports_comments_in_format_string() {
    let temp = TempDir::new().unwrap();
    let t = temp.child("some");
    CairoPluginProjectBuilder::default()
        .add_primitive_token_dep()
        .lib_rs(indoc! {r##"
        use cairo_lang_macro::{ProcMacroResult, TokenStream, inline_macro, quote_format};
        #[inline_macro]
        pub fn some(_token_stream: TokenStream) -> ProcMacroResult {
          let tokens = quote_format! {
            r#"/// Doc comment.
            ///
            // Regular comment.
            pub fn foo() -> felt252 {{
              // Function body comment.
              21
            }}"#
          };
          ProcMacroResult::new(tokens)
        }
        "##})
        .build(&t);

    let project = temp.child("hello");
    ProjectBuilder::start()
        .name("hello")
        .version("1.0.0")
        .dep("some", &t)
        .lib_cairo(indoc! {r#"
            some!();

            #[executable]
            fn main() -> felt252 {
                foo()
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
                #[executable]
                fn main() -> felt252 {
                    foo()
                }
                /// Doc comment.
                ///
                // Regular comment.
                pub fn foo() -> felt252 {
                    // Function body comment.
                    21
                }
            }
        "#},
    );
}

#[test]
fn quote_format_macro_supports_comments_in_token_stream() {
    let temp = TempDir::new().unwrap();
    let t = temp.child("some");
    CairoPluginProjectBuilder::default()
        .add_primitive_token_dep()
        .add_cairo_lang_parser_dep()
        .add_cairo_lang_syntax_dep()
        .lib_rs(indoc! {r##"
        use cairo_lang_macro::{ProcMacroResult, TokenStream, attribute_macro, quote_format};
        use cairo_lang_macro::{TokenTree, Token, TextSpan};
        use cairo_lang_parser::utils::SimpleParserDatabase;
        use cairo_lang_syntax::node::with_db::SyntaxNodeWithDb;
        #[attribute_macro]
        pub fn some(_attr: TokenStream, token_stream: TokenStream) -> ProcMacroResult {
          let db_val = SimpleParserDatabase::default();
          let db = &db_val;
          let (body, _diagnostics) = db.parse_token_stream(&token_stream);
          let name = TokenTree::Ident(Token::new("new_module", TextSpan::call_site()));
          let body = SyntaxNodeWithDb::new(&body, db);
          let tokens = quote_format! {
            r#"mod {} {{
                {}
            }}"#,
            name,
            body
          };
          ProcMacroResult::new(tokens)
        }
      "##})
        .build(&t);
    let project = temp.child("hello");
    ProjectBuilder::start()
        .name("hello")
        .version("1.0.0")
        .dep("some", &t)
        .lib_cairo(indoc! {r#"
            #[some]
            /// Doc comment.
            fn main() -> u32 {
              // completly wrong type
              true
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
                mod new_module {
                    /// Doc comment.
                    fn main() -> u32 {
                        // completly wrong type
                        true
                    }
                }
            }
        "#},
    );
}

#[test]
fn quote_macro_with_cairo_specific_syntax() {
    let temp = TempDir::new().unwrap();
    let t = temp.child("some");
    CairoPluginProjectBuilder::default().add_primitive_token_dep()
        .add_cairo_lang_parser_dep()
        .add_cairo_lang_syntax_dep()
        .lib_rs(indoc! {r##"
        use cairo_lang_macro::{ProcMacroResult, TokenStream, attribute_macro, quote};
        use cairo_lang_parser::utils::SimpleParserDatabase;
        use cairo_lang_syntax::node::with_db::SyntaxNodeWithDb;
        #[attribute_macro]
        pub fn some(_attr: TokenStream, _token_stream: TokenStream) -> ProcMacroResult {
          let db_val = SimpleParserDatabase::default();
          let db = &db_val;
          let code = r#"
              #[derive(Drop)]
              struct Rectangle {
                  width: u64,
                  height: u64,
              }
              #[derive(Drop, PartialEq)]
              struct Square {
                  side_length: u64,
              }
              impl RectangleIntoSquare of TryInto<Rectangle, Square> {
                  fn try_into(self: Rectangle) -> Option<Square> {
                      if self.height == self.width {
                          Option::Some(Square { side_length: self.height })
                      } else {
                          Option::None
                      }
                  }
              }
              fn main() {
                let rectangle = Rectangle { width: 8, height: 8 };
                let result: Square = rectangle.try_into().unwrap();
                let expected = Square { side_length: 8 };
                assert!(
                    result == expected,
                    "Rectangle with equal width and height should be convertible to a square."
                );
                let rectangle = Rectangle { width: 5, height: 8 };
                let result: Option<Square> = rectangle.try_into();
                assert!(
                    result.is_none(),
                    "Rectangle with different width and height should not be convertible to a square."
                );
              }
          "#;
          let syntax_node = db.parse_virtual(code).unwrap();
          let syntax_node_with_db = SyntaxNodeWithDb::new(&syntax_node, db);
          let tokens = quote! {
            #syntax_node_with_db
            trait Circle {
              fn print() -> ();
            }
            impl CircleImpl of Circle {
              fn print() -> () {
                println!("This is a circle!");
              }
            }
          };
          ProcMacroResult::new(tokens)
        }
      "##})
        .build(&t);
    let project = temp.child("hello");
    ProjectBuilder::start()
        .name("hello")
        .version("1.0.0")
        .dep("some", &t)
        .lib_cairo(indoc! {r#"
            #[some]
            fn main() -> u32 {
              // completly wrong type
              true
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
                  #[derive(Drop)]
                  struct Rectangle {
                      width: u64,
                      height: u64,
                  }
                  #[derive(Drop, PartialEq)]
                  struct Square {
                      side_length: u64,
                  }
                  impl RectangleIntoSquare of TryInto<Rectangle, Square> {
                      fn try_into(self: Rectangle) -> Option<Square> {
                          if self.height == self.width {
                              Option::Some(Square { side_length: self.height })
                          } else {
                              Option::None
                          }
                      }
                  }
                  fn main() {
                      let rectangle = Rectangle { width: 8, height: 8 };
                      let result: Square = rectangle.try_into().unwrap();
                      let expected = Square { side_length: 8 };
                      assert!(
                          result == expected,
                          "Rectangle with equal width and height should be convertible to a square.",
                      );
                      let rectangle = Rectangle { width: 5, height: 8 };
                      let result: Option<Square> = rectangle.try_into();
                      assert!(
                          result.is_none(),
                          "Rectangle with different width and height should not be convertible to a square.",
                      );
                  }
                  trait Circle {
                      fn print() -> ();
                  }
                  impl CircleImpl of Circle {
                      fn print() -> () {
                          println!("This is a circle!");
                      }
                  }
                  impl RectangleDrop<> of core::traits::Drop<Rectangle>;
                  impl SquareDrop<> of core::traits::Drop<Square>;
                  impl SquarePartialEq<> of core::traits::PartialEq<Square> {
                      fn eq(lhs: @Square, rhs: @Square) -> bool {
                          core::traits::PartialEq::<u64>::eq(lhs.side_length, rhs.side_length)
                      }
                  }
              }
          "#},
    );
}

#[test]
fn quote_macro_parse_incoming_token_stream() {
    let temp = TempDir::new().unwrap();
    let t = temp.child("some");
    CairoPluginProjectBuilder::default()
        .add_primitive_token_dep()
        .add_cairo_lang_parser_dep()
        .add_cairo_lang_syntax_dep()
        .lib_rs(indoc! {r##"
        use cairo_lang_macro::{ProcMacroResult, TokenStream, attribute_macro, quote};
        use cairo_lang_macro::{TokenTree, Token, TextSpan};
        use cairo_lang_parser::utils::SimpleParserDatabase;
        use cairo_lang_syntax::node::with_db::SyntaxNodeWithDb;
        #[attribute_macro]
        pub fn some(_attr: TokenStream, token_stream: TokenStream) -> ProcMacroResult {
          let db_val = SimpleParserDatabase::default();
          let db = &db_val;
          let (body, _diagnostics) = db.parse_token_stream(&token_stream);
          let name = TokenTree::Ident(Token::new("new_module", TextSpan::call_site()));
          let body = SyntaxNodeWithDb::new(&body, db);
          let tokens = quote! {
            mod #name {
                #body
            }
          };
          ProcMacroResult::new(tokens)
        }
      "##})
        .build(&t);
    let project = temp.child("hello");
    ProjectBuilder::start()
        .name("hello")
        .version("1.0.0")
        .dep("some", &t)
        .lib_cairo(indoc! {r#"
            #[some]
            fn main() -> u32 {
              // completly wrong type
              true
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
                mod new_module {
                    fn main() -> u32 {
                        // completly wrong type
                        true
                    }
                }
            }
        "#},
    );

    Scarb::quick_command()
        .arg("check")
        // Disable output from Cargo.
        .env("CARGO_TERM_QUIET", "true")
        .current_dir(&project)
        .assert()
        .failure()
        .stdout_eq(indoc! {r#"
            [..]Compiling some v1.0.0 ([..]Scarb.toml)
            [..]Checking hello v1.0.0 ([..]Scarb.toml)
            error: Unexpected return type. Expected: "core::integer::u32", found: "core::bool".
             --> [..]lib.cairo:2:14
            fn main() -> u32 {
                         ^^^

            error: could not check `hello` due to [..] previous error
        "#});
}

#[test]
fn quote_macro_with_token_interpolation() {
    let temp = TempDir::new().unwrap();
    let t = temp.child("some");
    CairoPluginProjectBuilder::default()
        .add_primitive_token_dep()
        .lib_rs(indoc! {r##"
            use cairo_lang_macro::{ProcMacroResult, TokenStream, attribute_macro, TokenTree, Token, TextSpan, quote};
            #[attribute_macro]
            pub fn some(_attr: TokenStream, _token_stream: TokenStream) -> ProcMacroResult {
                let name_string = "MyStruct".to_string();
                let name_token = TokenTree::Ident(Token::new(name_string.clone(), TextSpan::call_site()));
                let impl_string = format!("{}NameImpl", name_string);
                let impl_token = TokenTree::Ident(Token::new(impl_string, TextSpan::call_site()));
                let res_string = format!("\"{}\"", name_string);
                let res_token = TokenTree::Ident(Token::new(res_string, TextSpan::call_site()));
                let tokens = quote! {
                    impl #impl_token of NameTrait<#name_token> {
                        fn name(self: @#name_token) -> ByteArray {
                            #res_token
                        }
                    }
                };
                ProcMacroResult::new(tokens)
            }
        "##})
        .build(&t);
    let project = temp.child("hello");
    ProjectBuilder::start()
        .name("hello")
        .version("1.0.0")
        .dep("some", &t)
        .lib_cairo(indoc! {r#"
            pub trait NameTrait<T> {
                fn name(self: @NameTrait) -> ByteArray;
            }
            pub struct MyStruct {}

            #[some]
            fn main() -> u32 {
               true
            }
        "#})
        .build(&project);

    Scarb::quick_command()
        .arg("expand")
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
                pub trait NameTrait<T> {
                    fn name(self: @NameTrait) -> ByteArray;
                }
                pub struct MyStruct {}
                impl MyStructNameImpl of NameTrait<MyStruct> {
                    fn name(self: @MyStruct) -> ByteArray {
                        "MyStruct"
                    }
                }
            }
        "#},
    );
}

#[test]
fn quote_macro_preserves_spans_of_parsed_args() {
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

    Scarb::quick_command()
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
fn quote_format_macro() {
    let temp = TempDir::new().unwrap();
    let t = temp.child("some");
    CairoPluginProjectBuilder::default()
        .add_primitive_token_dep()
        .lib_rs(indoc! {r##"
            use cairo_lang_macro::{ProcMacroResult, TokenStream, inline_macro, TokenTree, Token, TextSpan, quote_format};
            
            #[inline_macro]
            pub fn some(_token_stream: TokenStream) -> ProcMacroResult {
                let x = TokenTree::Ident(Token::new("2".to_string(), TextSpan::call_site()));
                let tokens = quote_format! {
                    r#"assert(1 + 1 == {}, 'fail')"#,
                    x
                };
                ProcMacroResult::new(tokens)
            }
        "##})
        .build(&t);

    let project = temp.child("hello");
    ProjectBuilder::start()
        .name("hello")
        .version("1.0.0")
        .dep_cairo_execute()
        .manifest_extra(indoc! {r#"
            [executable]
            [cairo]
            enable-gas = false
        "#})
        .dep("some", &t)
        .lib_cairo(indoc! {r#"
            #[executable]
            fn main() -> felt252 {
                some!();
                42
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
        .stdout_eq(indoc! {r#"
        [..] Compiling some v1.0.0 [..]
        [..] Compiling hello v1.0.0 [..]
        [..] Finished `dev` profile [..]
        [..]Executing hello
        Program output:
        42
        Saving output to: target/execute/hello/execution1
        "#})
        .success();
}

#[test]
fn quote_format_macro_with_code_block() {
    let temp = TempDir::new().unwrap();
    let t = temp.child("some");
    CairoPluginProjectBuilder::default()
        .add_primitive_token_dep()
        .lib_rs(indoc! {r##"
            use cairo_lang_macro::{ProcMacroResult, TokenStream, inline_macro, TokenTree, Token, TextSpan, quote_format};

            #[inline_macro]
            pub fn some(_token_stream: TokenStream) -> ProcMacroResult {
                let fn_name = TokenTree::Ident(Token::new("foo".to_string(), TextSpan::call_site()));
                let x = TokenTree::Ident(Token::new("42".to_string(), TextSpan::call_site()));
                let tokens = quote_format! {
                    r#"pub fn {}() -> felt252 {{
                        return {};
                    }}"#, fn_name, x
                };
                ProcMacroResult::new(tokens)
            }
        "##})
        .build(&t);

    let project = temp.child("hello");
    ProjectBuilder::start()
        .name("hello")
        .version("1.0.0")
        .dep_cairo_execute()
        .manifest_extra(indoc! {r#"
            [executable]
            [cairo]
            enable-gas = false
        "#})
        .dep("some", &t)
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
        // Disable output from Cargo.
        .env("CARGO_TERM_QUIET", "true")
        .current_dir(&project)
        .assert()
        .stdout_eq(indoc! {r#"
        [..] Compiling some v1.0.0 [..]
        [..] Compiling hello v1.0.0 [..]
        [..] Finished `dev` profile [..]
        [..]Executing hello
        Program output:
        42
        Saving output to: target/execute/hello/execution1
        "#})
        .success();
}

#[test]
fn quote_format_macro_no_args() {
    let temp = TempDir::new().unwrap();
    let t = temp.child("some");
    CairoPluginProjectBuilder::default()
        .add_primitive_token_dep()
        .lib_rs(indoc! {r##"
            use cairo_lang_macro::{ProcMacroResult, TokenStream, inline_macro, TokenTree, Token, TextSpan, quote_format};

            #[inline_macro]
            pub fn some(_token_stream: TokenStream) -> ProcMacroResult {
                let x = TokenTree::Ident(Token::new("42".to_string(), TextSpan::call_site()));
                let tokens = quote_format! {"{}", x};
                ProcMacroResult::new(tokens)
            }
        "##})
        .build(&t);

    let project = temp.child("hello");
    ProjectBuilder::start()
        .name("hello")
        .version("1.0.0")
        .dep_cairo_execute()
        .manifest_extra(indoc! {r#"
            [executable]
            [cairo]
            enable-gas = false
        "#})
        .dep("some", &t)
        .lib_cairo(indoc! {r#"
            #[executable]
            fn main() -> felt252 { some!() }
        "#})
        .build(&project);

    Scarb::quick_command()
        .arg("execute")
        .arg("--print-program-output")
        // Disable output from Cargo.
        .env("CARGO_TERM_QUIET", "true")
        .current_dir(&project)
        .assert()
        .stdout_eq(indoc! {r#"
        [..] Compiling some v1.0.0 [..]
        [..] Compiling hello v1.0.0 [..]
        [..] Finished `dev` profile [..]
        [..]Executing hello
        Program output:
        42
        Saving output to: target/execute/hello/execution1
        "#})
        .success();
}

#[test]
fn quote_format_macro_multiple_args() {
    let temp = TempDir::new().unwrap();
    let t = temp.child("some");
    CairoPluginProjectBuilder::default()
        .add_primitive_token_dep()
        .lib_rs(indoc! {r##"
            use cairo_lang_macro::{ProcMacroResult, TokenStream, inline_macro, TokenTree, Token, TextSpan, quote_format};

            #[inline_macro]
            pub fn some(_token_stream: TokenStream) -> ProcMacroResult {
                let a = TokenTree::Ident(Token::new("21".to_string(), TextSpan::call_site()));
                let b = TokenTree::Ident(Token::new("42".to_string(), TextSpan::call_site()));
                let tokens = quote_format! {
                    "{} + {}",
                    a,
                    b
                };
                ProcMacroResult::new(tokens)
            }
        "##})
        .build(&t);
    let project = temp.child("hello");
    ProjectBuilder::start()
        .name("hello")
        .version("1.0.0")
        .dep_cairo_execute()
        .manifest_extra(indoc! {r#"
            [executable]
            [cairo]
            enable-gas = false
        "#})
        .dep("some", &t)
        .lib_cairo(indoc! {r#"
            #[executable]
            fn main() -> felt252 { some!() }
        "#})
        .build(&project);

    Scarb::quick_command()
        .arg("execute")
        .arg("--print-program-output")
        // Disable output from Cargo.
        .env("CARGO_TERM_QUIET", "true")
        .current_dir(&project)
        .assert()
        .stdout_eq(indoc! {r#"
        [..] Compiling some v1.0.0 [..]
        [..] Compiling hello v1.0.0 [..]
        [..] Finished `dev` profile [..]
        [..]Executing hello
        Program output:
        63
        Saving output to: target/execute/hello/execution1
        "#})
        .success();
}

#[test]
fn quote_format_macro_fails_on_invalid_syntax() {
    let temp = TempDir::new().unwrap();
    let t = temp.child("some");
    CairoPluginProjectBuilder::default()
        .add_primitive_token_dep()
        .lib_rs(indoc! {r##"
            use cairo_lang_macro::{ProcMacroResult, TokenStream, inline_macro, quote_format};
            #[inline_macro]
            pub fn some(_token_stream: TokenStream) -> ProcMacroResult {
                // Missing closing parenthesis
                let tokens = quote_format! {"assert(false, 'fail'"};
                ProcMacroResult::new(tokens)
            }
        "##})
        .build(&t);

    let project = temp.child("hello");
    ProjectBuilder::start()
        .name("hello")
        .version("1.0.0")
        .dep_cairo_execute()
        .manifest_extra(indoc! {r#"
            [executable]
            [cairo]
            enable-gas = false
        "#})
        .dep("some", &t)
        .lib_cairo(indoc! {r#"
            #[executable]
            fn main() -> felt252 { some!() }
        "#})
        .build(&project);

    Scarb::quick_command()
        .arg("execute")
        .arg("--print-program-output")
        // Disable output from Cargo.
        .env("CARGO_TERM_QUIET", "true")
        .current_dir(&project)
        .assert()
        .failure()
        .stdout_eq(indoc! {r#"
        [..]Compiling some v1.0.0 [..]
        [..]Compiling hello v1.0.0 [..]
        error: Parser error in macro-expanded code: Missing token ')'.
         --> [..]src/lib.cairo:2:24
        fn main() -> felt252 { some!() }
                               ^^^^^^^
        
        error: could not compile `hello` due to 1 previous error
        error: `scarb` command exited with error
        "#})
        .failure();
}

#[test]
fn quote_format_macro_with_indexed_args() {
    let temp = TempDir::new().unwrap();
    let t = temp.child("some");
    CairoPluginProjectBuilder::default()
        .add_primitive_token_dep()
        .lib_rs(indoc! {r##"
            use cairo_lang_macro::{ProcMacroResult, TokenStream, inline_macro, TokenTree, Token, TextSpan, quote_format};

            #[inline_macro]
            pub fn some(_token_stream: TokenStream) -> ProcMacroResult {
                let first = TokenTree::Ident(Token::new("1".to_string(), TextSpan::call_site()));
                let second = TokenTree::Ident(Token::new("100".to_string(), TextSpan::call_site()));
                let tokens = quote_format! {
                    "{1} - {0}",
                    first,
                    second
                };
                ProcMacroResult::new(tokens)
            }
        "##})
        .build(&t);

    let project = temp.child("hello");
    ProjectBuilder::start()
        .name("hello")
        .version("1.0.0")
        .dep_cairo_execute()
        .manifest_extra(indoc! {r#"
            [executable]
            [cairo]
            enable-gas = false
        "#})
        .dep("some", &t)
        .lib_cairo(indoc! {r#"
            #[executable]
            fn main() -> felt252 { some!() }
        "#})
        .build(&project);

    Scarb::quick_command()
        .arg("execute")
        .arg("--print-program-output")
        // Disable output from Cargo.
        .env("CARGO_TERM_QUIET", "true")
        .current_dir(&project)
        .assert()
        .stdout_eq(indoc! {r#"
        [..] Compiling some v1.0.0 [..]
        [..] Compiling hello v1.0.0 [..]
        [..] Finished `dev` profile [..]
        [..]Executing hello
        Program output:
        99
        Saving output to: target/execute/hello/execution1
        "#})
        .success();
}

#[test]
fn quote_format_macro_fails_on_named_args() {
    let temp = TempDir::new().unwrap();
    let t = temp.child("some");
    CairoPluginProjectBuilder::default()
        .add_primitive_token_dep()
        .lib_rs(indoc! {r##"
            use cairo_lang_macro::{ProcMacroResult, TokenStream, inline_macro, TokenTree, Token, TextSpan, quote_format};

            #[inline_macro]
            pub fn some(_token_stream: TokenStream) -> ProcMacroResult {
                let x = TokenTree::Ident(Token::new("42".to_string(), TextSpan::call_site()));
                let tokens = quote_format! {"{name}", x };
                ProcMacroResult::new(tokens)
            }
        "##})
        .build(&t);

    let project = temp.child("hello");
    ProjectBuilder::start()
        .name("hello")
        .version("1.0.0")
        .dep("some", &t)
        .lib_cairo(indoc! {r#"
            #[executable]
            fn main() -> felt252 { some!() }
        "#})
        .build(&project);

    Scarb::quick_command()
        .arg("check")
        // Disable output from Cargo.
        .env("CARGO_TERM_QUIET", "true")
        // Disable colors in Cargo output.
        .env("CARGO_TERM_COLOR", "never")
        .current_dir(&project)
        .assert()
        .failure()
        .stdout_eq(indoc! {r#"
        [..]Compiling some v1.0.0 [..]
        error: named placeholder '{name}' is not supported by this macro.
               help: use positional ('{}') or indexed placeholders ('{0}', '{1}', ...) instead.
         --> src/lib.rs:6:33
          |
        6 |     let tokens = quote_format! {"{name}", x };
          |                                 ^^^^^^^^
        error: could not compile `some` (lib) due to 1 previous error
        error: process did not exit successfully: exit [..]: 101
        error: could not compile `some` due to 1 previous error
        "#});
}

#[test]
fn quote_format_macro_fails_on_invalid_index() {
    let temp = TempDir::new().unwrap();
    let t = temp.child("some");
    CairoPluginProjectBuilder::default()
        .add_primitive_token_dep()
        .lib_rs(indoc! {r##"
            use cairo_lang_macro::{ProcMacroResult, TokenStream, inline_macro, TokenTree, Token, TextSpan, quote_format};

            #[inline_macro]
            pub fn some(_token_stream: TokenStream) -> ProcMacroResult {
                let a = TokenTree::Ident(Token::new("10".to_string(), TextSpan::call_site()));
                let b = TokenTree::Ident(Token::new("20".to_string(), TextSpan::call_site()));
                let c = TokenTree::Ident(Token::new("30".to_string(), TextSpan::call_site()));
                let tokens = quote_format! {"{3}", a, b, c};
                ProcMacroResult::new(tokens)
            }
        "##})
        .build(&t);

    let project = temp.child("hello");
    ProjectBuilder::start()
        .name("hello")
        .version("1.0.0")
        .dep("some", &t)
        .lib_cairo(indoc! {r#"
            #[executable]
            fn main() -> felt252 { some!() }
        "#})
        .build(&project);

    Scarb::quick_command()
        .arg("check")
        // Disable output from Cargo.
        .env("CARGO_TERM_QUIET", "true")
        // Disable colors in Cargo output.
        .env("CARGO_TERM_COLOR", "never")
        .current_dir(&project)
        .assert()
        .failure()
        .stdout_eq(indoc! {r#"
        [..]Compiling some v1.0.0 [..]
        error: format arg index 3 is out of range (the format string contains 3 args).
         --> src/lib.rs:8:33
          |
        8 |     let tokens = quote_format! {"{3}", a, b, c};
          |                                 ^^^^^
        error: could not compile `some` (lib) due to 1 previous error
        error: process did not exit successfully: exit [..]: 101
        error: could not compile `some` due to 1 previous error
        "#});
}

#[test]
fn quote_format_macro_preserves_spans_of_parsed_args() {
    let temp = TempDir::new().unwrap();
    let t = temp.child("some");
    CairoPluginProjectBuilder::default()
        .add_primitive_token_dep()
        .add_cairo_lang_parser_dep()
        .add_cairo_lang_syntax_dep()
        .lib_rs(indoc! {r#"
            use cairo_lang_macro::{attribute_macro, quote_format, ProcMacroResult, TokenStream};
            use cairo_lang_parser::utils::SimpleParserDatabase;
            use cairo_lang_syntax::node::with_db::SyntaxNodeWithDb;

            #[attribute_macro]
            pub fn simple_attr(_args: TokenStream, item: TokenStream) -> ProcMacroResult {
                let db_val = SimpleParserDatabase::default();
                let db = &db_val;
                let (body, _diagnostics) = db.parse_token_stream(&item);
                let body = SyntaxNodeWithDb::new(&body, db);
                let ts = quote_format! {
                    "{}",
                    body
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

    Scarb::quick_command()
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
