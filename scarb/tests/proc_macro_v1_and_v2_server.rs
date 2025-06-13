use assert_fs::TempDir;
use assert_fs::prelude::PathChild;
use cairo_lang_macro::{TextSpan, Token, TokenStream as TokenStreamV2, TokenTree};
use scarb_proc_macro_server_types::methods::CodeOrigin::Span;
use scarb_proc_macro_server_types::methods::expand::ExpandAttribute;
use scarb_proc_macro_server_types::methods::expand::ExpandAttributeParams;
use scarb_proc_macro_server_types::methods::expand::ExpandDerive;
use scarb_proc_macro_server_types::methods::expand::ExpandDeriveParams;
use scarb_proc_macro_server_types::methods::expand::ExpandInline;
use scarb_proc_macro_server_types::methods::expand::ExpandInlineMacroParams;
use scarb_proc_macro_server_types::methods::{CodeMapping, CodeOrigin};
use scarb_proc_macro_server_types::scope::ProcMacroScope;
use scarb_test_support::cairo_plugin_project_builder::CairoPluginProjectBuilder;
use scarb_test_support::proc_macro_server::{ProcMacroClient, SIMPLE_MACROS_V1, SIMPLE_MACROS_V2};
use scarb_test_support::project_builder::ProjectBuilder;
use std::path::PathBuf;

fn setup_project_with_v1_and_v2_macro_deps(
    temp_dir: &TempDir,
    v1_macros_extra: Option<&str>,
    v2_macros_extra: Option<&str>,
) -> PathBuf {
    let plugin_package_v1 = temp_dir.child("some_v1");
    let v1_macros_str = v1_macros_extra.unwrap_or("");
    CairoPluginProjectBuilder::default_v1()
        .name("some_v1")
        .lib_rs(format!("{SIMPLE_MACROS_V1}\n{v1_macros_str}"))
        .build(&plugin_package_v1);

    let plugin_package_v2 = temp_dir.child("some_v2");
    let v2_macros_str = v2_macros_extra.unwrap_or("");
    CairoPluginProjectBuilder::default()
        .name("some_v2")
        .lib_rs(format!("{SIMPLE_MACROS_V2}\n{v2_macros_str}"))
        .build(&plugin_package_v2);

    let project = temp_dir.child("test_package");

    ProjectBuilder::start()
        .name("test_package")
        .version("1.0.0")
        .lib_cairo("")
        .dep("some_v1", plugin_package_v1)
        .dep("some_v2", plugin_package_v2)
        .build(&project);

    project.to_path_buf()
}

#[test]
fn defined_macros() {
    let t = TempDir::new().unwrap();
    let project = setup_project_with_v1_and_v2_macro_deps(&t, None, None);

    let mut proc_macro_client = ProcMacroClient::new(&project);

    let defined_macros = proc_macro_client.defined_macros_for_package("test_package");

    assert_eq!(
        &defined_macros.attributes,
        &["some_v1".to_string(), "some_v2".to_string()]
    );
    assert_eq!(
        &defined_macros.derives,
        &["some_derive_v1".to_string(), "some_derive_v2".to_string()]
    );
    assert_eq!(
        &defined_macros.inline_macros,
        &["inline_some_v1".to_string(), "inline_some_v2".to_string()]
    );
    assert_eq!(
        &defined_macros.executables,
        &[
            "some_executable_v1".to_string(),
            "some_executable_v2".to_string()
        ]
    );
}

#[test]
fn expand_attribute() {
    let replace_12_with_34_v1 = r#"
        #[attribute_macro]
        pub fn replace_12_with_34_v1(_attr: TokenStream, token_stream: TokenStream) -> ProcMacroResult {{
            let content = token_stream.to_string().replace("12", "34");
            ProcMacroResult::new(TokenStream::new(content))
        }}
    "#;

    let replace_12_with_34_v2 = r#"
        #[attribute_macro]
        pub fn replace_12_with_34_v2(_attr: TokenStream, token_stream: TokenStream) -> ProcMacroResult {{
            let new_token_string = token_stream.to_string().replace("12", "34");
            let token_stream = TokenStream::new(vec![TokenTree::Ident(Token::new(
                new_token_string.clone(),
                TextSpan { start: 0, end: new_token_string.len() as u32 },
            ))]);
            ProcMacroResult::new(token_stream)
        }}
    "#;
    let t = TempDir::new().unwrap();
    let project = setup_project_with_v1_and_v2_macro_deps(
        &t,
        Some(replace_12_with_34_v1),
        Some(replace_12_with_34_v2),
    );

    let mut proc_macro_client = ProcMacroClient::new(&project);

    for macro_name in ["replace_12_with_34_v1", "replace_12_with_34_v2"] {
        let component = proc_macro_client
            .defined_macros_for_package("test_package")
            .component;

        let code = "fn some_test_fn_12(){}".to_string();
        let span = TextSpan::new(0, code.len() as u32);
        let item = TokenStreamV2::new(vec![TokenTree::Ident(Token::new(code, span.clone()))]);

        let response = proc_macro_client
            .request_and_wait::<ExpandAttribute>(ExpandAttributeParams {
                context: ProcMacroScope {
                    component: component.clone(),
                },
                attr: macro_name.to_string(),
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

        if macro_name == "replace_12_with_34_v2" {
            assert!(response.code_mappings.is_some());
            assert_eq!(
                response.code_mappings.unwrap(),
                vec![
                    CodeMapping {
                        span: TextSpan { start: 0, end: 22 },
                        origin: CodeOrigin::Span(TextSpan { start: 0, end: 22 })
                    },
                    CodeMapping {
                        span: TextSpan { start: 0, end: 22 },
                        origin: CodeOrigin::CallSite(TextSpan { start: 0, end: 22 })
                    }
                ]
            );
        } else {
            assert!(response.code_mappings.is_none());
        }
    }
}

#[test]
fn expand_derive() {
    let t = TempDir::new().unwrap();
    let project = setup_project_with_v1_and_v2_macro_deps(&t, None, None);

    let mut proc_macro_client = ProcMacroClient::new(&project);

    let component = proc_macro_client
        .defined_macros_for_package("test_package")
        .component;

    for macro_name in ["some_derive_v1", "some_derive_v2"] {
        let code = "fn some_test_fn(){}".to_string();
        let span = TextSpan::new(0, code.len() as u32);
        let item = TokenStreamV2::new(vec![TokenTree::Ident(Token::new(code, span.clone()))]);

        let response = proc_macro_client
            .request_and_wait::<ExpandDerive>(ExpandDeriveParams {
                context: ProcMacroScope {
                    component: component.clone(),
                },
                derives: vec![macro_name.to_string()],
                item,
                call_site: span,
            })
            .unwrap();

        assert_eq!(response.diagnostics, vec![]);
        assert_eq!(
            response.token_stream.to_string(),
            "impl SomeImpl of SomeTrait {}".to_string()
        );

        if macro_name == "some_derive_v2" {
            assert!(response.code_mappings.is_some());
            assert_eq!(
                response.code_mappings.unwrap(),
                vec![
                    CodeMapping {
                        span: TextSpan { start: 0, end: 0 },
                        origin: Span(TextSpan { start: 0, end: 0 })
                    },
                    CodeMapping {
                        span: TextSpan { start: 0, end: 29 },
                        origin: Span(TextSpan { start: 0, end: 29 })
                    },
                    CodeMapping {
                        span: TextSpan { start: 0, end: 29 },
                        origin: CodeOrigin::CallSite(TextSpan { start: 0, end: 19 })
                    }
                ]
            );
        } else {
            assert_eq!(
                response.code_mappings,
                Some(vec![CodeMapping {
                    span: TextSpan { start: 0, end: 29 },
                    origin: Span(TextSpan { start: 0, end: 19 })
                },])
            );
        }
    }
}

#[test]
fn expand_inline() {
    let replace_all_15_with_25_v1 = r#"
        #[inline_macro]
        pub fn replace_all_15_with_25_v1(token_stream: TokenStream) -> ProcMacroResult {
            let content = token_stream.to_string().replace("15", "25");
            ProcMacroResult::new(TokenStream::new(content))
        }
    "#;

    let replace_all_15_with_25_v2 = r#"
        #[inline_macro]
        pub fn replace_all_15_with_25_v2(token_stream: TokenStream) -> ProcMacroResult {
            let content = token_stream.to_string().replace("15", "25");
            let token_stream = TokenStream::new(vec![TokenTree::Ident(Token::new(
                content.clone(),
                TextSpan { start: 0, end: content.len() as u32 },
            ))]);
            ProcMacroResult::new(token_stream)
        }
    "#;
    let t = TempDir::new().unwrap();
    let project = setup_project_with_v1_and_v2_macro_deps(
        &t,
        Some(replace_all_15_with_25_v1),
        Some(replace_all_15_with_25_v2),
    );

    let mut proc_macro_client = ProcMacroClient::new(&project);

    let component = proc_macro_client
        .defined_macros_for_package("test_package")
        .component;

    let args_code = "struct A { field: 15, other_field: macro_call!(12)}".to_string();
    let span = TextSpan::new(0, args_code.len() as u32);
    let args = TokenStreamV2::new(vec![TokenTree::Ident(Token::new(args_code, span.clone()))]);

    for macro_name in ["replace_all_15_with_25_v1", "replace_all_15_with_25_v2"] {
        let response = proc_macro_client
            .request_and_wait::<ExpandInline>(ExpandInlineMacroParams {
                context: ProcMacroScope {
                    component: component.clone(),
                },
                name: macro_name.to_string(),
                args: args.clone(),
                call_site: span.clone(),
            })
            .unwrap();

        assert_eq!(response.diagnostics, vec![]);
        assert_eq!(
            response.token_stream.to_string(),
            "struct A { field: 25, other_field: macro_call!(12)}".to_string()
        );

        if macro_name == "replace_all_15_with_25_v2" {
            assert!(response.code_mappings.is_some());
            assert_eq!(
                response.code_mappings.unwrap(),
                vec![
                    CodeMapping {
                        span: TextSpan { start: 0, end: 51 },
                        origin: Span(TextSpan { start: 0, end: 51 })
                    },
                    CodeMapping {
                        span: TextSpan { start: 0, end: 51 },
                        origin: CodeOrigin::CallSite(TextSpan { start: 0, end: 51 })
                    }
                ]
            );
        } else {
            assert!(response.code_mappings.is_none())
        }
    }
}
