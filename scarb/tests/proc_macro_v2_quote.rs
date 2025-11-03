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

    Scarb::quick_snapbox()
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

    Scarb::quick_snapbox()
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

    Scarb::quick_snapbox()
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
                    5
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
                mod new_module {
                    fn main() -> u32 {
                        // completly wrong type
                        true
                    }
                }
            }
        "#},
    );

    Scarb::quick_snapbox()
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

    Scarb::quick_snapbox()
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

    Scarb::quick_snapbox()
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

    Scarb::quick_snapbox()
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

    Scarb::quick_snapbox()
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

    Scarb::quick_snapbox()
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

    Scarb::quick_snapbox()
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

    Scarb::quick_snapbox()
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
        error: process did not exit successfully: exit status: 101
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

    Scarb::quick_snapbox()
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
        error: process did not exit successfully: exit status: 101
        error: could not compile `some` due to 1 previous error
        "#});
}
