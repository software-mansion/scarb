use assert_fs::TempDir;
use assert_fs::fixture::PathChild;
use indoc::indoc;
use scarb_test_support::cairo_plugin_project_builder::CairoPluginProjectBuilder;
use scarb_test_support::command::Scarb;
use scarb_test_support::fsx::ChildPathEx;
use scarb_test_support::project_builder::ProjectBuilder;

#[test]
fn can_use_quote() {
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
        .stdout_matches(indoc! {r#"
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
fn can_use_quote_with_token_tree() {
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
        .stdout_matches(indoc! {r#"
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
fn can_use_quote_with_token_stream() {
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
        .stdout_matches(indoc! {r#"
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
fn can_use_quote_with_syntax_node() {
    let temp = TempDir::new().unwrap();
    let t = temp.child("some");
    CairoPluginProjectBuilder::default()
        .add_primitive_token_dep()
        .add_dep(r#"cairo-lang-syntax = "2.9.1""#)
        .add_dep(r#"cairo-lang-parser = "2.9.1""#)
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

    snapbox::assert_eq(
        indoc! {r#"
            mod hello {
                fn main() -> felt252 {
                    5
                }
            }
        "#},
        expanded,
    );
}

#[test]
fn can_use_quote_with_cairo_specific_syntax() {
    let temp = TempDir::new().unwrap();
    let t = temp.child("some");
    CairoPluginProjectBuilder::default().add_primitive_token_dep()
        .add_dep(r#"cairo-lang-syntax = "2.9.1""#)
        .add_dep(r#"cairo-lang-parser = "2.9.1""#)
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

    snapbox::assert_eq(
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
        expanded,
    );
}

#[test]
fn can_parse_incoming_token_stream() {
    let temp = TempDir::new().unwrap();
    let t = temp.child("some");
    CairoPluginProjectBuilder::default()
        .add_primitive_token_dep()
        .add_dep(r#"cairo-lang-syntax = { git = "https://github.com/starkware-libs/cairo", rev = "b5fdf14a8bd2e4973e2adcec17abf1ae5c1ddfdc" }"#)
        .add_dep(r#"cairo-lang-parser = { git = "https://github.com/starkware-libs/cairo", rev = "b5fdf14a8bd2e4973e2adcec17abf1ae5c1ddfdc" }"#)
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

    snapbox::assert_eq(
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
        expanded,
    );

    Scarb::quick_snapbox()
        .arg("check")
        // Disable output from Cargo.
        .env("CARGO_TERM_QUIET", "true")
        .current_dir(&project)
        .assert()
        .failure()
        .stdout_matches(indoc! {r#"
            [..]Compiling some v1.0.0 ([..]Scarb.toml)
            [..]Checking hello v1.0.0 ([..]Scarb.toml)
            error: Unexpected return type. Expected: "core::integer::u32", found: "core::bool".
             --> [..]lib.cairo:2:14
            fn main() -> u32 {
                         ^^^^

            error: could not check `hello` due to previous error
        "#});
}

#[test]
fn can_parse_with_token_interpolation() {
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

    snapbox::assert_eq(
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
        expanded,
    );
}
