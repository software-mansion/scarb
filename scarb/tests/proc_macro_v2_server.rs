use assert_fs::TempDir;
use assert_fs::prelude::PathChild;
use cairo_lang_macro::{TextSpan, Token, TokenStream as TokenStreamV2, TokenTree};
use scarb_proc_macro_server_types::methods::expand::ExpandAttribute;
use scarb_proc_macro_server_types::methods::expand::ExpandAttributeParams;
use scarb_proc_macro_server_types::methods::expand::ExpandDerive;
use scarb_proc_macro_server_types::methods::expand::ExpandDeriveParams;
use scarb_proc_macro_server_types::methods::expand::ExpandInline;
use scarb_proc_macro_server_types::methods::expand::ExpandInlineMacroParams;
use scarb_proc_macro_server_types::scope::ProcMacroScope;
use scarb_test_support::cairo_plugin_project_builder::CairoPluginProjectBuilder;
use scarb_test_support::proc_macro_server::{ProcMacroClient, SIMPLE_MACROS_V2};

use scarb_test_support::project_builder::ProjectBuilder;

#[test]
fn defined_macros() {
    let t = TempDir::new().unwrap();
    let plugin_package = t.child("some");

    CairoPluginProjectBuilder::default()
        .lib_rs(SIMPLE_MACROS_V2)
        .build(&plugin_package);

    let project = t.child("test_package");

    ProjectBuilder::start()
        .name("test_package")
        .version("1.0.0")
        .lib_cairo("")
        .dep("some", plugin_package)
        .build(&project);

    let mut proc_macro_client = ProcMacroClient::new(&project);

    let defined_macros = proc_macro_client.defined_macros_for_package("test_package");

    assert_eq!(&defined_macros.attributes, &["some".to_string()]);
    assert_eq!(&defined_macros.derives, &["some_derive".to_string()]);
    assert_eq!(&defined_macros.inline_macros, &["inline_some".to_string()]);
    assert_eq!(
        &defined_macros.executables,
        &["some_executable".to_string()]
    );
}

#[test]
fn expand_attribute() {
    let t = TempDir::new().unwrap();
    let plugin_package = t.child("some");

    let replace_12_with_34 = r#"
        #[attribute_macro]
        pub fn replace_12_with_34(_attr: TokenStream, token_stream: TokenStream) -> ProcMacroResult {{
            let new_token_string = token_stream.to_string().replace("12", "34");
            let token_stream = TokenStream::new(vec![TokenTree::Ident(Token::new(
                new_token_string.clone(),
                TextSpan { start: 0, end: new_token_string.len() as u32 },
            ))]);
            ProcMacroResult::new(token_stream)
        }}
    "#;

    CairoPluginProjectBuilder::default()
        .lib_rs(format!("{SIMPLE_MACROS_V2}\n{replace_12_with_34}"))
        .build(&plugin_package);

    let project = t.child("test_package");

    ProjectBuilder::start()
        .name("test_package")
        .version("1.0.0")
        .lib_cairo("")
        .dep("some", plugin_package)
        .build(&project);

    let mut proc_macro_client = ProcMacroClient::new(&project);

    let component = proc_macro_client
        .defined_macros_for_package("test_package")
        .component;

    let code = "fn some_test_fn_12(){}".to_string();
    let span = TextSpan::new(0, code.len() as u32);
    let item = TokenStreamV2::new(vec![TokenTree::Ident(Token::new(code, span.clone()))]);

    let response = proc_macro_client
        .request_and_wait::<ExpandAttribute>(ExpandAttributeParams {
            context: ProcMacroScope { component },
            attr: "replace_12_with_34".to_string(),
            args: TokenStreamV2::empty(),
            item,
            call_site: span,
        })
        .unwrap();

    assert_eq!(response.diagnostics, vec![]);
    assert_eq!(
        response.token_stream.to_string(),
        "fn some_test_fn_34(){}".to_string()
    );
}

#[test]
fn expand_derive() {
    let t = TempDir::new().unwrap();
    let plugin_package = t.child("some");

    CairoPluginProjectBuilder::default()
        .lib_rs(SIMPLE_MACROS_V2)
        .build(&plugin_package);

    let project = t.child("test_package");

    ProjectBuilder::start()
        .name("test_package")
        .version("1.0.0")
        .lib_cairo("")
        .dep("some", plugin_package)
        .build(&project);

    let mut proc_macro_client = ProcMacroClient::new(&project);

    let component = proc_macro_client
        .defined_macros_for_package("test_package")
        .component;

    let code = "fn some_test_fn(){}".to_string();
    let span = TextSpan::new(0, code.len() as u32);
    let item = TokenStreamV2::new(vec![TokenTree::Ident(Token::new(code, span.clone()))]);

    let response = proc_macro_client
        .request_and_wait::<ExpandDerive>(ExpandDeriveParams {
            context: ProcMacroScope { component },
            derives: vec!["some_derive".to_string()],
            item,
            call_site: span,
        })
        .unwrap();

    assert_eq!(response.diagnostics, vec![]);
    assert_eq!(
        response.token_stream.to_string(),
        "impl SomeImpl of SomeTrait {}".to_string()
    );
}

#[test]
fn expand_inline() {
    let t = TempDir::new().unwrap();
    let plugin_package = t.child("some");

    let replace_all_15_with_25 = r#"
        #[inline_macro]
        pub fn replace_all_15_with_25(token_stream: TokenStream) -> ProcMacroResult {
            let content = token_stream.to_string().replace("15", "25");
            let token_stream = TokenStream::new(vec![TokenTree::Ident(Token::new(
                content.clone(),
                TextSpan { start: 0, end: content.len() as u32 },
            ))]);
            ProcMacroResult::new(token_stream)
        }
    "#;

    CairoPluginProjectBuilder::default()
        .lib_rs(format!("{SIMPLE_MACROS_V2}\n{replace_all_15_with_25}"))
        .build(&plugin_package);

    let project = t.child("test_package");

    ProjectBuilder::start()
        .name("test_package")
        .version("1.0.0")
        .lib_cairo("")
        .dep("some", plugin_package)
        .build(&project);

    let mut proc_macro_client = ProcMacroClient::new(&project);

    let component = proc_macro_client
        .defined_macros_for_package("test_package")
        .component;

    let args_code = "struct A { field: 15 , other_field: macro_call!(12)}".to_string();
    let span = TextSpan::new(0, args_code.len() as u32);
    let args = TokenStreamV2::new(vec![TokenTree::Ident(Token::new(args_code, span.clone()))]);

    let response = proc_macro_client
        .request_and_wait::<ExpandInline>(ExpandInlineMacroParams {
            context: ProcMacroScope { component },
            name: "replace_all_15_with_25".to_string(),
            args,
            call_site: span,
        })
        .unwrap();

    assert_eq!(response.diagnostics, vec![]);
    assert_eq!(
        response.token_stream.to_string(),
        "struct A { field: 25 , other_field: macro_call!(12)}".to_string()
    );
}
