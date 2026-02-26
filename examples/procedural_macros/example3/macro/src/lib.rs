use cairo_lang_macro::{
    attribute_macro, quote, ProcMacroResult, TextSpan, Token, TokenStream, TokenTree,
};
use cairo_lang_parser::utils::SimpleParserDatabase;
use cairo_lang_syntax::node::{
    ast::{self, ModuleItem},
    helpers::HasName,
    kind::SyntaxKind,
    with_db::SyntaxNodeWithDb,
    SyntaxNode, Terminal, TypedSyntaxNode,
};

#[attribute_macro]
fn create_wrapper(args: TokenStream, body: TokenStream) -> ProcMacroResult {
    // Initialize parser to parse function body.
    let db = SimpleParserDatabase::default();
    // Define small helper for creating single token.
    let new_token = |content| TokenTree::Ident(Token::new(content, TextSpan::call_site()));
    // Parse attribute arguments with helper function.
    let (wrapper_name, argument_value) = parse_arguments(&db, args);
    let wrapper_name = new_token(wrapper_name);
    let argument_value = new_token(argument_value);
    // Parse incoming token stream.
    let (node, _diagnostics) = db.parse_token_stream(&body);
    // Parse function name.
    let function_name = parse_function_name(&db, node.clone());
    let function_name = new_token(function_name);
    // Create `SyntaxNodeWithDb`, from a single syntax node.
    // This struct implements `ToPrimitiveTokenStream` trait, thus can be used as argument to `quote!`.
    let node = SyntaxNodeWithDb::new(&node, &db);
    ProcMacroResult::new(quote! {
        #node

        fn #wrapper_name() -> u32 {
            #function_name(#argument_value)
        }
    })
}

fn parse_function_name<'db>(db: &'db SimpleParserDatabase, node: SyntaxNode<'db>) -> String {
    assert_eq!(node.kind(db), SyntaxKind::SyntaxFile);
    let file = ast::SyntaxFile::from_syntax_node(db, node);
    let items = file.items(db).elements_vec(db);
    assert_eq!(items.len(), 1);
    let func = items.into_iter().next().unwrap();
    assert!(matches!(func, ModuleItem::FreeFunction(_)));
    let ModuleItem::FreeFunction(func) = func else {
        panic!("not a function");
    };
    func.name(db).text(db).to_string(db)
}

fn parse_arguments(db: &SimpleParserDatabase, args: TokenStream) -> (String, String) {
    // Parse argument token stream.
    let (node, _diagnostics) = db.parse_token_stream_expr(&args);
    // Read parsed syntax node.
    assert_eq!(node.kind(db), SyntaxKind::ExprListParenthesized);
    let expr = ast::ExprListParenthesized::from_syntax_node(db, node);
    // `expressions` returns a list of all expressions inside parentheses.
    // We expect two expressions, the first one is the wrapper name, the second one is the argument value.
    let mut expressions = expr.expressions(db).elements_vec(db).into_iter();
    let wrapper_name_expr = expressions.next().unwrap();
    let wrapper_name = wrapper_name_expr.as_syntax_node().get_text(db).to_string();
    let value_expr = expressions.next().unwrap();
    let value = value_expr.as_syntax_node().get_text(db).to_string();
    // We return both expressions as strings.
    (wrapper_name, value)
}
