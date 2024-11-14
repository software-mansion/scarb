use cairo_lang_macro::{TextSpan, Token, TokenStream, TokenTree};
use cairo_lang_parser::utils::SimpleParserDatabase;
use cairo_lang_syntax::node::db::SyntaxGroup;
use cairo_lang_utils::Upcast;
// use cairo_lang_utils::Upcast;

use crate::quote_format;
use indoc::indoc;

#[test]
fn test_plain_code() {
    let token_stream = quote_format!("fn main() { }");

    let expected = TokenStream::new(vec![
        TokenTree::Ident(Token::new("fn".to_string(), None)),
        TokenTree::Ident(Token::new(" ".to_string(), None)),
        TokenTree::Ident(Token::new("main()".to_string(), None)),
        TokenTree::Ident(Token::new(" ".to_string(), None)),
        TokenTree::Ident(Token::new("{".to_string(), None)),
        TokenTree::Ident(Token::new(" ".to_string(), None)),
        TokenTree::Ident(Token::new("}".to_string(), None)),
    ]);

    assert_eq!(token_stream, expected);
}

#[test]
fn test_syntax_node() {
    let db_val = SimpleParserDatabase::default();
    let db = &db_val;
    let code = indoc! {"
      fn main() {
        let a = 5;
      }
    "};

    let root_syntax_node = db.parse_virtual(code).unwrap();
    let module_item_list = db.get_children(root_syntax_node.clone());
    let item_list = db.get_children(module_item_list.first().unwrap().clone());
    let function_items = db.get_children(item_list.first().unwrap().clone());
    let expr_block_children = db.get_children(function_items.get(3).unwrap().clone());
    let statment = expr_block_children.get(1).unwrap();

    let result = quote_format!(
        db,
        indoc! {"
      fn main() {
        let b = 6;
        let c = 7;
        {}
      }
    "},
        statment
    );

    let expected = TokenStream::new(vec![
        TokenTree::Ident(Token::new("fn".to_string(), None)),
        TokenTree::Ident(Token::new(" ".to_string(), None)),
        TokenTree::Ident(Token::new("main()".to_string(), None)),
        TokenTree::Ident(Token::new(" ".to_string(), None)),
        TokenTree::Ident(Token::new("{\n".to_string(), None)),
        TokenTree::Ident(Token::new(" ".to_string(), None)),
        TokenTree::Ident(Token::new(" ".to_string(), None)),
        TokenTree::Ident(Token::new("let".to_string(), None)),
        TokenTree::Ident(Token::new(" ".to_string(), None)),
        TokenTree::Ident(Token::new("b".to_string(), None)),
        TokenTree::Ident(Token::new(" ".to_string(), None)),
        TokenTree::Ident(Token::new("=".to_string(), None)),
        TokenTree::Ident(Token::new(" ".to_string(), None)),
        TokenTree::Ident(Token::new("6;\n".to_string(), None)),
        TokenTree::Ident(Token::new(" ".to_string(), None)),
        TokenTree::Ident(Token::new(" ".to_string(), None)),
        TokenTree::Ident(Token::new("let".to_string(), None)),
        TokenTree::Ident(Token::new(" ".to_string(), None)),
        TokenTree::Ident(Token::new("c".to_string(), None)),
        TokenTree::Ident(Token::new(" ".to_string(), None)),
        TokenTree::Ident(Token::new("=".to_string(), None)),
        TokenTree::Ident(Token::new(" ".to_string(), None)),
        TokenTree::Ident(Token::new("7;\n".to_string(), None)),
        TokenTree::Ident(Token::new(" ".to_string(), None)),
        TokenTree::Ident(Token::new(" ".to_string(), None)),
        TokenTree::Ident(Token::new(
            "  let a = 5;\n".to_string(),
            Some(TextSpan::new(12, 25)),
        )),
        TokenTree::Ident(Token::new("\n}\n".to_string(), None)),
    ]);

    assert_eq!(result, expected);

    // println!("{}", print_tree(db, statment, false, true));
    println!("{:?}", result);
}
