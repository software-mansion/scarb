use assert_fs::fixture::PathChild;
use assert_fs::TempDir;
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
        .dep("some", &t)
        .lib_cairo(indoc! {r#"
            fn main() -> felt252 { some!() }
        "#})
        .build(&project);

    Scarb::quick_snapbox()
        .arg("cairo-run")
        // Disable output from Cargo.
        .env("CARGO_TERM_QUIET", "true")
        .current_dir(&project)
        .assert()
        .stdout_matches(indoc! {r#"
        [..] Compiling some v1.0.0 [..]
        [..] Compiling hello v1.0.0 [..]
        [..] Finished `dev` profile [..]
        [..] Running hello
        Run completed successfully, returning [5]
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
        use cairo_lang_macro::{ProcMacroResult, TokenStream, inline_macro, TokenTree, Token, quote};
        #[inline_macro]
        pub fn some(_token_stream: TokenStream) -> ProcMacroResult {
          let token = TokenTree::Ident(Token::new("5".to_string(), None));
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
        .dep("some", &t)
        .lib_cairo(indoc! {r#"
            fn main() -> felt252 {
              some!()
            }
        "#})
        .build(&project);

    Scarb::quick_snapbox()
        .arg("cairo-run")
        // Disable output from Cargo.
        .env("CARGO_TERM_QUIET", "true")
        .current_dir(&project)
        .assert()
        .stdout_matches(indoc! {r#"
        [..] Compiling some v1.0.0 [..]
        [..] Compiling hello v1.0.0 [..]
        [..] Finished `dev` profile [..]
        [..] Running hello
        Run completed successfully, returning [5]
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
        use cairo_lang_macro::{ProcMacroResult, TokenStream, inline_macro, TokenTree, Token, quote};
        #[inline_macro]
        pub fn some(_token_stream: TokenStream) -> ProcMacroResult {
          let token = TokenStream::new(vec![TokenTree::Ident(Token::new("5".to_string(), None))]);
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
        .dep("some", &t)
        .lib_cairo(indoc! {r#"
            fn main() -> felt252 {
              some!()
            }
        "#})
        .build(&project);

    Scarb::quick_snapbox()
        .arg("cairo-run")
        // Disable output from Cargo.
        .env("CARGO_TERM_QUIET", "true")
        .current_dir(&project)
        .assert()
        .stdout_matches(indoc! {r#"
        [..] Compiling some v1.0.0 [..]
        [..] Compiling hello v1.0.0 [..]
        [..] Finished `dev` profile [..]
        [..] Running hello
        Run completed successfully, returning [5]
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
                  impl RectangleDrop of core::traits::Drop<Rectangle>;
                  impl SquareDrop of core::traits::Drop<Square>;
                  impl SquarePartialEq of core::traits::PartialEq<Square> {
                      fn eq(lhs: @Square, rhs: @Square) -> bool {
                          lhs.side_length == rhs.side_length
                      }
                  }
              }
          "#},
        expanded,
    );
}
