use cairo_lang_macro::{
    Diagnostics, ProcMacroResult, TextSpan, Token, TokenStream, TokenTree, attribute_macro, quote,
};
use cairo_lang_parser::utils::SimpleParserDatabase;
use cairo_lang_syntax::node::{
    SyntaxNode, Terminal, TypedSyntaxNode,
    ast::{self, ModuleItem},
    helpers::HasName,
    kind::SyntaxKind,
    with_db::SyntaxNodeWithDb,
};

#[attribute_macro]
fn create_wrapper(args: TokenStream, body: TokenStream) -> ProcMacroResult {
    let db = SimpleParserDatabase::default();
    let new_token = |content| TokenTree::Ident(Token::new(content, TextSpan::call_site()));

    let (wrapper_name, argument_value) = match parse_arguments(&db, args) {
        Ok((name, value)) => (name, value),
        Err(diag) => return ProcMacroResult::new(body).with_diagnostics(diag),
    };

    let wrapper_name = new_token(wrapper_name);
    let argument_value = new_token(argument_value);

    let (node, _diagnostics) = db.parse_token_stream(&body);

    let function_name = match parse_function_name(&db, node.clone()) {
        Ok(name) => name,
        Err(diag) => return ProcMacroResult::new(body).with_diagnostics(diag),
    };

    let function_name = new_token(function_name);
    let node = SyntaxNodeWithDb::new(&node, &db);

    ProcMacroResult::new(quote! {
      #node

      fn #wrapper_name() -> u32 {
        #function_name(#argument_value)
      }
    })
}

fn parse_function_name<'db>(
    db: &'db SimpleParserDatabase,
    node: SyntaxNode<'db>,
) -> Result<String, Diagnostics> {
    if node.kind(db) != SyntaxKind::SyntaxFile {
        return Err(Diagnostics::new(Vec::new()).error("Expected SyntaxFile"));
    }
    let file = ast::SyntaxFile::from_syntax_node(db, node);
    let items = file.items(db).elements_vec(db);
    if items.len() != 1 {
        return Err(Diagnostics::new(Vec::new()).error("Expected exactly one item"));
    }
    let func = items.into_iter().next().unwrap();
    match func {
        ModuleItem::FreeFunction(f) => Ok(f.name(db).text(db).to_string(db)),
        _ => Err(Diagnostics::new(Vec::new()).error("Expected a function")),
    }
}

fn parse_arguments(
    db: &SimpleParserDatabase,
    args: TokenStream,
) -> Result<(String, String), Diagnostics> {
    let (node, _diagnostics) = db.parse_token_stream_expr(&args);
    if node.kind(db) != SyntaxKind::ExprListParenthesized {
        return Err(Diagnostics::new(Vec::new()).error("Expected parenthesized expression list"));
    }
    let expr = ast::ExprListParenthesized::from_syntax_node(db, node);
    let mut expressions = expr.expressions(db).elements_vec(db).into_iter();

    let wrapper_name_expr = match expressions.next() {
        Some(e) => e,
        None => return Err(Diagnostics::new(Vec::new()).error("Expected wrapper name argument")),
    };
    let wrapper_name = wrapper_name_expr.as_syntax_node().get_text(db).to_string();

    let value_expr = match expressions.next() {
        Some(e) => e,
        None => return Err(Diagnostics::new(Vec::new()).error("Expected value argument")),
    };
    let value = value_expr.as_syntax_node().get_text(db).to_string();

    Ok((wrapper_name, value))
}
