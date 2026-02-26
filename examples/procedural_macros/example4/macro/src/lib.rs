use cairo_lang_macro::{Diagnostics, ProcMacroResult, TokenStream, attribute_macro, quote};
use cairo_lang_parser::utils::SimpleParserDatabase;
use cairo_lang_syntax::node::{TypedSyntaxNode, ast, with_db::SyntaxNodeWithDb};

#[attribute_macro]
fn my_macro(_args: TokenStream, body: TokenStream) -> ProcMacroResult {
    let diagnostics = Diagnostics::new(Vec::new());
    // Initialize parser and parse the incoming token stream.
    let db = SimpleParserDatabase::default();
    // Parse incoming token stream.
    let (node, _diagnostics) = db.parse_token_stream(&body);

    // Locate the function item this attribute macro is applied to.
    let module_item_list = match node.get_children(&db).get(0) {
        Some(item) => item,
        None => {
            let diagnostics =
                diagnostics.error("This attribute macro should be only used for a function");
            return ProcMacroResult::new(body).with_diagnostics(diagnostics);
        }
    };

    let function = match module_item_list.get_children(&db).get(0) {
        Some(item) => item,
        None => {
            let diagnostics =
                diagnostics.error("This attribute macro should be only used for a function");
            return ProcMacroResult::new(body).with_diagnostics(diagnostics);
        }
    };

    // Extract the function's syntax components.
    let expr = ast::FunctionWithBody::from_syntax_node(&db, *function);
    let attributes = expr.attributes(&db);
    let visibility = expr.visibility(&db);
    let declaration = expr.declaration(&db);
    let body_expr = expr.body(&db);

    // Pull out braces and the first two statements from the body.
    let l_brace = body_expr.lbrace(&db);
    let r_brace = body_expr.rbrace(&db);
    let mut statements = body_expr.statements(&db).elements(&db);
    let first_statement = match statements.next() {
        Some(stmt) => stmt,
        None => {
            let diagnostics = diagnostics
                .error("function needs at least 2 statements to be valid candidate for attr macro");
            return ProcMacroResult::new(body).with_diagnostics(diagnostics);
        }
    };
    let second_statement = match statements.next() {
        Some(stmt) => stmt,
        None => {
            let diagnostics = diagnostics
                .error("function needs at least 2 statements to be valid candidate for attr macro");
            return ProcMacroResult::new(body).with_diagnostics(diagnostics);
        }
    };

    // Convert syntax nodes into `SyntaxNodeWithDb` for quoting.
    let attributes_node = attributes.as_syntax_node();
    let visibility_node = visibility.as_syntax_node();
    let declaration_node = declaration.as_syntax_node();
    let l_brace_node = l_brace.as_syntax_node();
    let r_brace_node = r_brace.as_syntax_node();
    let first_statement_node = first_statement.as_syntax_node();
    let second_statement_node = second_statement.as_syntax_node();

    let attributes_result = SyntaxNodeWithDb::new(&attributes_node, &db);
    let visibility_result = SyntaxNodeWithDb::new(&visibility_node, &db);
    let declaration_result = SyntaxNodeWithDb::new(&declaration_node, &db);
    let l_brace_result = SyntaxNodeWithDb::new(&l_brace_node, &db);
    let r_brace_result = SyntaxNodeWithDb::new(&r_brace_node, &db);
    let first_statement_result = SyntaxNodeWithDb::new(&first_statement_node, &db);
    let second_statement_result = SyntaxNodeWithDb::new(&second_statement_node, &db);

    // Rebuild the function, injecting a statement between the first two.
    ProcMacroResult::new(quote! {
      #attributes_result
      #visibility_result #declaration_result #l_brace_result
      #first_statement_result
      let macro_variable: felt252 = 2;
      #second_statement_result
      #r_brace_result
    })
}
