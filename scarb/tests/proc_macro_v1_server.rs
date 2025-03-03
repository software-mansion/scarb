use assert_fs::prelude::PathChild;
use assert_fs::TempDir;
use cairo_lang_macro_v2::{TextSpan, Token, TokenStream, TokenTree};
use scarb_proc_macro_server_types::methods::expand::*;
use scarb_proc_macro_server_types::scope::ProcMacroScope;
use scarb_test_support::cairo_plugin_project_builder::CairoPluginProjectBuilder;
use scarb_test_support::proc_macro_server::DefinedMacrosInfo;
use scarb_test_support::proc_macro_server::ProcMacroClient;
use scarb_test_support::proc_macro_server::SIMPLE_MACROS_V1;
use scarb_test_support::project_builder::ProjectBuilder;

#[test]
fn defined_macros() {
    let t = TempDir::new().unwrap();
    let plugin_package = t.child("some");

    CairoPluginProjectBuilder::default_v1()
        .lib_rs(SIMPLE_MACROS_V1)
        .build(&plugin_package);

    let project = t.child("test_package");

    ProjectBuilder::start()
        .name("test_package")
        .version("1.0.0")
        .lib_cairo("")
        .dep("some", plugin_package)
        .build(&project);

    let mut proc_macro_client = ProcMacroClient::new(&project);

    let DefinedMacrosInfo { defined_macros, .. } =
        proc_macro_client.defined_macros_for_package("test_package");

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

    let rename_to_very_new_name = r##"
        #[attribute_macro]
        pub fn rename_to_very_new_name(_attr: TokenStream, token_stream: TokenStream) -> ProcMacroResult {{
            let re = regex::Regex::new(r#"fn (\w+)\(.*\)\{.*\}"#).unwrap();
            let input = token_stream.to_string();
            let name = re.captures(&input).unwrap().get(1).unwrap().as_str();

            let output = input.replace(name, "very_new_name");

            ProcMacroResult::new(TokenStream::new(output))
        }}
    "##;

    CairoPluginProjectBuilder::default_v1()
        .lib_rs(format!("{SIMPLE_MACROS_V1}\n{rename_to_very_new_name}"))
        .add_dep(r#"regex = "1.11.1""#)
        .build(&plugin_package);

    let project = t.child("test_package");

    ProjectBuilder::start()
        .name("test_package")
        .version("1.0.0")
        .lib_cairo("")
        .dep("some", plugin_package)
        .build(&project);

    let mut proc_macro_client = ProcMacroClient::new(&project);

    let DefinedMacrosInfo { package_id, .. } =
        proc_macro_client.defined_macros_for_package("test_package");

    let response = proc_macro_client
        .request_and_wait::<ExpandAttribute>(ExpandAttributeParams {
            context: ProcMacroScope { package_id },
            attr: "rename_to_very_new_name".to_string(),
            args: TokenStream::empty(),
            call_site: TextSpan::new(0, 0),
            item: TokenStream::new(vec![TokenTree::Ident(Token::new(
                "fn some_test_fn(){}",
                TextSpan::new(0, 0),
            ))]),
        })
        .unwrap();

    assert_eq!(response.diagnostics, vec![]);
    assert_eq!(
        response.token_stream.to_string(),
        "fn very_new_name(){}".to_string()
    );
}

#[test]
fn expand_derive() {
    let t = TempDir::new().unwrap();
    let plugin_package = t.child("some");

    CairoPluginProjectBuilder::default_v1()
        .lib_rs(SIMPLE_MACROS_V1)
        .build(&plugin_package);

    let project = t.child("test_package");

    ProjectBuilder::start()
        .name("test_package")
        .version("1.0.0")
        .lib_cairo("")
        .dep("some", plugin_package)
        .build(&project);

    let mut proc_macro_client = ProcMacroClient::new(&project);

    let DefinedMacrosInfo { package_id, .. } =
        proc_macro_client.defined_macros_for_package("test_package");

    let item = TokenStream::new(vec![TokenTree::Ident(Token::new(
        "fn some_test_fn(){}",
        TextSpan::new(0, 0),
    ))]);

    let response = proc_macro_client
        .request_and_wait::<ExpandDerive>(ExpandDeriveParams {
            context: ProcMacroScope { package_id },
            derives: vec!["some_derive".to_string()],
            call_site: TextSpan::new(0, 0),
            item,
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
            ProcMacroResult::new(TokenStream::new(token_stream.to_string().replace("15", "25")))
        }
    "#;

    CairoPluginProjectBuilder::default_v1()
        .lib_rs(format!("{SIMPLE_MACROS_V1}\n{replace_all_15_with_25}"))
        .build(&plugin_package);

    let project = t.child("test_package");

    ProjectBuilder::start()
        .name("test_package")
        .version("1.0.0")
        .lib_cairo("")
        .dep("some", plugin_package)
        .build(&project);

    let mut proc_macro_client = ProcMacroClient::new(&project);

    let DefinedMacrosInfo { package_id, .. } =
        proc_macro_client.defined_macros_for_package("test_package");

    let response = proc_macro_client
        .request_and_wait::<ExpandInline>(ExpandInlineMacroParams {
            context: ProcMacroScope { package_id },
            name: "replace_all_15_with_25".to_string(),
            call_site: TextSpan::new(0, 0),
            args: TokenStream::new(vec![TokenTree::Ident(Token::new(
                "struct A { field: 15 , other_field: macro_call!(12)}",
                TextSpan::new(0, 0),
            ))]),
        })
        .unwrap();

    assert_eq!(response.diagnostics, vec![]);
    assert_eq!(
        response.token_stream.to_string(),
        "struct A { field: 25 , other_field: macro_call!(12)}".to_string()
    );
}
