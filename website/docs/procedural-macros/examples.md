<script setup>
import { data as rel } from "../../github.data";
</script>

# Examples

## Example 1: returning a value

Note, we omit the toml files here, as their content is the same as in the previous example.

Usually you want to define a procedural macro that injects some code into your Cairo project.
In this example, we will create an inline procedural macro that returns a single numerical value as a token.

```rust
use cairo_lang_macro::{inline_macro, ProcMacroResult, TextSpan, Token, TokenStream, TokenTree};

#[inline_macro]
pub fn fib(args: TokenStream) -> ProcMacroResult {
    let argument = parse_arguments(args);

    let result = fib(argument);

    ProcMacroResult::new(TokenStream::new(vec![TokenTree::Ident(Token::new(
        result.to_string(),
        TextSpan::call_site(),
    ))]))
}

/// Parse argument into a numerical value.
///
/// Always expects a single, numerical value in parentheses.
/// Panics otherwise.
fn parse_arguments(args: TokenStream) -> u32 {
    let args = args.to_string();
    let (_prefix, rest) = args.split_once("(").unwrap();
    let (argument, _suffix) = rest.rsplit_once(")").unwrap();
    let argument = argument.parse::<u32>().unwrap();
    argument
}

/// Calculate n-th Fibonacci number.
fn fib(mut n: u32) -> u32 {
    let mut a: u32 = 0;
    let mut b: u32 = 1;
    while n != 0 {
        n = n - 1;
        let temp = b;
        b = a + b;
        a = temp;
    }
    a
}
```

This example is a bit more complex than the previous one.
The macro works in three steps:

1. Parse inline macro arguments.
2. Perform some computation in Rust.
3. Construct and return a new `TokenStream` as a result.

The first step is done by the `parse_arguments` function, in a very primitive way.
We convert the whole input `TokenStream` into a single string and then look for left and right parentheses.
We always assume the argument to be a single numerical value.

> [!WARNING]
> This function is only useful for demonstration.
> In reality, you should make your parser more robust and never should assume that the input is valid.
> Properly handling parsing errors is a must if you want your users to understand why their code is not compiling.
> Please see [parsing token stream](./parsing) for more information.

We then call the `fib` function, which calculates a number in Fibonacci sequence.
Note that this calculation happens **during the compilation**, when the procedural macro expansion happens, not during
the Cairo program execution.

The result is a single numerical value, that we convert to a `TokenStream`, by wrapping it in three subsequent abstractions:
`Token`, `TokenTree` and `TokenStream`.
`Token` represents a single Cairo token, and consists of two parts: a string representing the token content and a span.
Span is a location in the source code of a project that uses this macro.
It is used to persist information about the origin of tokens that are moved or copied from user code.
For new tokens, that you create in your macro like we do here, it should be set to `TextSpan::call_site()`, which is
a span that points to the location of the macro call.
`TokenTree` is an additional enum that describes the type of the token, currently only `TokenTree::Ident` is used (but
may be more in the future).
Finally, `TokenStream` is a stream of `TokenTree`s, that can be iterated over or converted into a string.

Then you can use this macro in your Cairo code:
Note that `fib!(16)` actually calls the `fib` inline macro we defined before.

```cairo
fn main() -> u32 {
    fib!(16)
}

#[cfg(test)]
mod tests {
    use super::main;

    #[test]
    fn it_works() {
        assert(main() == 987, 'invalid value returned!');
    }
}
```

If you test your program with `scarb test`, it works:

```
Collected 1 test(s) from hello_world package
Running 1 test(s) from src/
[PASS] hello_world::tests::it_works (l1_gas: ~0, l1_data_gas: ~0, l2_gas: ~40000)
Tests: 1 passed, 0 failed, 0 ignored, 0 filtered out
```

Notice how no computations actually happen during Cairo program execution.
This Cairo project compiles into the following CASM code:

```
[ap + 0] = 987, ap++;
ret;
```

> [!INFO]
> To see a real life example of a procedural macro that offloads some work into compile time,
> you can take a look at the [`alexandria` project](https://github.com/keep-starknet-strange/alexandria/tree/6b98da52c819aeb86697b787b4bcf4abe94bc788/packages/macros).

## Example 2: building token stream with `quote!` macro

In our macro, we manually construct the token stream we return.
This approach is fine for basic and very short results, like the single numerical value we return, but it does
not scale very well for longer results.
Constructing longer token streams this way, say a whole new function you want to return, would not be very convenient.

The `cairo-lang-macro` crate defines a [`quote!` macro](https://docs.rs/cairo-lang-macro/latest/cairo_lang_macro/macro.quote.html),
which can be used to build `TokenStream`s from Rust variables.
This acts as a convenient wrapper around creating and pushing tokens into a `TokenStream` manually.

For instance, if we decide we no longer want to return a single value from our macro, but rather create a const variable
declaration with it, we can use the `quote!` macro to make our implementation more concise.

We first change how we use our macro. The `main` function now returns `FIB16` constant, that will be later created by
the macro expansion. We move the macro call to the top level of the module.

```cairo
fib!(16);

fn main() -> u32 {
    FIB16
}
```

We also change the `fib` function to use the `quote!` macro.
Inside the macro call, we declare the constant value as if it was a normal Cairo source file.
When we want to substitute some Rust variable with its value, we can use its name prefixed with a hash sign `#`.

We can do this with any variable that implements [`ToPrimitiveTokenStream`](https://docs.rs/cairo-lang-primitive-token/latest/cairo_lang_primitive_token/trait.ToPrimitiveTokenStream.html)
trait from `cairo-lang-primitive-token` crate.
This trait is implemented for `TokenStream` itself, so we can use `quote!` for composition of multiple token streams.

```rust
#[inline_macro]
pub fn fib(args: TokenStream) -> ProcMacroResult {
    let argument = parse_arguments(args);

    let result = fib(argument);

    let result = TokenTree::Ident(Token::new(result.to_string(), TextSpan::call_site()));

    ProcMacroResult::new(quote! {
        const FIB16: u32 =  #result;
    })
}
```

In a similar manner, you can use syntax nodes from the `cairo-lang-syntax` AST as variables in the macro.
This is especially useful when you need to copy some Cairo code from the input token stream, say, some function annotated
with your attribute procedural macro.

```rust
use cairo_lang_macro::{attribute_macro, quote, ProcMacroResult};

#[attribute_macro]
fn attr_name() {
    // Parse incoming token stream.
    let db = SimpleParserDatabase::default();
    let (node, _diagnostics) = db.parse_token_stream(&body);
    // Create `SyntaxNodeWithDb`, from a single syntax node.
    // This struct implements `ToPrimitiveTokenStream` trait, thus can be used as argument to `quote!`.
    let node = SyntaxNodeWithDb::new(&node, &db);
    // Use the node in `quote!` macro.
    ProcMacroResult::new(quote! {
        #node
    })
}
```

## Example 3: creating a new function

Working example of this approach can be an attribute macro that creates a new function wrapper.
This new function will call the original function with some argument.
The name of the wrapper function and argument value will be controlled by attribute macro arguments.

```rust
// src/lib.rs
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
```

We can use the new attribute to generate a wrapper for our `fib` function.

```cairo
// hello_world/src/lib.cairo

fn main() -> u32 {
    named_wrapper()
}

#[create_wrapper(named_wrapper,16)]
fn fib(mut n: u32) -> u32 {
    let mut a: u32 = 0;
    let mut b: u32 = 1;
    while n != 0 {
        n = n - 1;
        let temp = b;
        b = a + b;
        a = temp;
    }
    a
}

#[cfg(test)]
mod tests {
    use super::main;

    #[test]
    fn it_works() {
        assert(main() == 987, 'invalid value returned!');
    }
}
```

Our test will ensure that the wrapper function can be called and returns the correct value.

```
Collected 1 test(s) from hello_world package
Running 1 test(s) from src/
[PASS] hello_world::tests::it_works (l1_gas: ~0, l1_data_gas: ~0, l2_gas: ~80000)
Tests: 1 passed, 0 failed, 0 ignored, 0 filtered out
```

## Example 4: modifying an already existing function

Using quote macro we can as well modify the body of the function that the attribute macro is being applied to.
In this example, with the use of an attribute macro, we will define completely new variable inside the function, which later will be used in a user code. We also make all the diagnostics from user code correctly mapped to the origin code.

```rust
// src/lib.rs
use cairo_lang_macro::{attribute_macro, quote, ProcMacroResult, TokenStream};
use cairo_lang_parser::{printer::print_tree, utils::SimpleParserDatabase};
use cairo_lang_syntax::node::{ast, with_db::SyntaxNodeWithDb, TypedSyntaxNode};

#[attribute_macro]
fn my_macro(_attr: TokenStream, code: TokenStream) -> ProcMacroResult {
    // Initialize parser and parse the incoming token stream.
    let db = SimpleParserDatabase::default();
    // Parse incoming token stream.
    let (node, _diagnostics) = db.parse_token_stream(&code);

    // This section is used only for macro debugging purposes.
    // This way, we can see the exact syntax structure of the item we want to modify.
    let node_tree = print_tree(&db, &node, false, false);
    println!("node tree: \n{}", node_tree);

    // Locate the function item this attribute macro is applied to.
    let module_item_list = node
        .get_children(&db)
        .get(0)
        .expect("This attribute macro should be only used for a function");

    let function = module_item_list
        .get_children(&db)
        .get(0)
        .expect("This attribute macro should be only used for a function");

    // Extract the function's syntax components.
    let expr = ast::FunctionWithBody::from_syntax_node(&db, *function);
    let attributes = expr.attributes(&db);
    let visibility = expr.visibility(&db);
    let declaration = expr.declaration(&db);
    let body = expr.body(&db);

    // Pull out braces and the first two statements from the body.
    let l_brace = body.lbrace(&db);
    let r_brace = body.rbrace(&db);
    let mut statements = body.statements(&db).elements(&db);
    let first_statement = statements
        .next()
        .expect("function needs at least 2 statements to be valid candidate for attr macro");
    let second_statement = statements
        .next()
        .expect("function needs at least 2 statements to be valid candidate for attr macro");

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
```

We can use the new attribute with a function, that will have an access to the new variable. The variable will be inserted right after the first original statement of the function.

```cairo
// hello_world/src/lib.cairo
#[my_macro]
fn example_function() {
    let _variable1: felt252 = 1;
    let _variable2: felt252 = macro_variable;
}
```

This way, we ensure that this code is fully valid and the `scarb check` will end with a success:

```
Finished checking `dev` profile target(s) in 1 second
```

Note that all the original user code used in the `quote!` macro will be correctly mapped to the origin code. If something would be wrong with the user code. For example, if user makes a mistake in its own code, like that:

```cairo
// hello_world/src/lib.cairo
#[my_macro]
fn example_function() {
    let _variable1: felt252 = non_existing_variable;
    let _variable2: felt252 = macro_variable;
}
```

we will get this error:

```
error[E0006]: Identifier not found.
 --> .../src/lib.cairo:3:31
    let _variable1: felt252 = non_existing_variable;
                              ^^^^^^^^^^^^^^^^^^^^^
note: this error originates in the attribute macro: `my_macro`
```

which is pointing to the original user code, as he's the one that's the author of this piece of code.

If we make a mistake while generating code in the macro like this:

```rust
// src/lib.rs
}
#[attribute_macro]
fn my_macro(_attr: TokenStream, code: TokenStream) -> ProcMacroResult {
  ...
  // Rebuild the function, injecting a statement between the first two.
  ProcMacroResult::new(quote! {
    #attributes_result
    #visibility_result #declaration_result #l_brace_result
    #first_statement_result
    let macro_variable: felt252 = total_nonsense;
    #second_statement_result
    #r_brace_result
  })
}
```

we are left with error, that maps directly to the attribute macro (which is correct, because it's the macro author's fault here):

```
error[E0006]: Identifier not found.
 --> .../src/lib.cairo:1:1
#[my_macro]
^^^^^^^^^^^
note: this error originates in the attribute macro: `my_macro`
```

Note that we are using the `cairo_lang_parser::printer::print_tree` function here. It's really helpful when creating any procedural macros that modify or read any syntax items. It's also important to note, that in some rather rare cases, the structure of the tree changes as the Cairo compiler is under continuous development.
