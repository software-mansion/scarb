use assert_fs::TempDir;
use assert_fs::prelude::PathChild;
use cairo_lang_macro::{TextSpan, Token, TokenStream as TokenStreamV2, TokenTree};
use scarb_proc_macro_server_types::methods::SpannedTokenStream;
use scarb_proc_macro_server_types::methods::expand::ExpandAttribute;
use scarb_proc_macro_server_types::methods::expand::ExpandAttributeParams;
use scarb_proc_macro_server_types::methods::expand::ExpandDerive;
use scarb_proc_macro_server_types::methods::expand::ExpandDeriveParams;
use scarb_proc_macro_server_types::methods::expand::ExpandInline;
use scarb_proc_macro_server_types::methods::expand::ExpandInlineMacroParams;
use scarb_proc_macro_server_types::scope::ProcMacroScope;
use scarb_proc_macro_server_types::scope::Workspace;
use scarb_test_support::cairo_plugin_project_builder::CairoPluginProjectBuilder;
use scarb_test_support::proc_macro_server::{ProcMacroClient, SIMPLE_MACROS_V1, SIMPLE_MACROS_V2};
use scarb_test_support::project_builder::ProjectBuilder;
use std::path::PathBuf;

/// Content and span of every token in the stream.
///
/// Spans are what the caller maps the expansion back onto the original source with. A v2 macro
/// sets them itself; an expansion coming from the v1 api, which has no spans, is reported as a
/// single token covering the whole origin.
fn tokens(token_stream: &SpannedTokenStream) -> Vec<(String, TextSpan)> {
    token_stream
        .0
        .iter()
        .map(|token| (token.content.clone(), token.span.clone()))
        .collect()
}

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

    let mut manifest_path = project.clone();
    manifest_path.push("test_package");
    manifest_path.set_file_name("Scarb.toml");

    let mut proc_macro_client = ProcMacroClient::new(&project);

    let defined_macros =
        proc_macro_client.defined_macros_for_package("test_package", manifest_path);

    assert_eq!(
        &defined_macros
            .attributes
            .into_iter()
            .map(|m| m.name)
            .collect::<Vec<_>>(),
        &["some_v1".to_string(), "some_v2".to_string()]
    );
    assert_eq!(
        &defined_macros
            .derives
            .into_iter()
            .map(|m| m.name)
            .collect::<Vec<_>>(),
        &["some_derive_v1".to_string(), "some_derive_v2".to_string()]
    );
    assert_eq!(
        &defined_macros
            .inline_macros
            .into_iter()
            .map(|m| m.name)
            .collect::<Vec<_>>(),
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

    let mut manifest_path = project.clone();
    manifest_path.push("test_package");
    manifest_path.set_file_name("Scarb.toml");

    let mut proc_macro_client = ProcMacroClient::new(&project);

    for macro_name in ["replace_12_with_34_v1", "replace_12_with_34_v2"] {
        let component = proc_macro_client
            .defined_macros_for_package("test_package", manifest_path.clone())
            .component;

        let code = "fn some_test_fn_12(){}".to_string();
        let span = TextSpan::new(0, code.len() as u32);
        let item = TokenStreamV2::new(vec![TokenTree::Ident(Token::new(code, span.clone()))]);

        let response = proc_macro_client
            .request_and_wait::<ExpandAttribute>(ExpandAttributeParams {
                context: ProcMacroScope {
                    workspace: Workspace {
                        manifest_path: manifest_path.clone(),
                    },
                    component: component.clone(),
                },
                attr: macro_name.to_string(),
                args: TokenStreamV2::empty(),
                item,
                adapted_call_site: span,
            })
            .unwrap();

        assert_eq!(response.diagnostics, vec![]);
        assert_eq!(
            response.token_stream.to_string(),
            "fn some_test_fn_34(){}".to_string()
        );

        // Both api versions report the expansion as a single token. The v2 macro sets that span
        // itself; for the v1 macro the server attributes the whole output to the whole input item.
        assert_eq!(
            tokens(&response.token_stream),
            vec![(
                "fn some_test_fn_34(){}".to_string(),
                TextSpan::new(0, "fn some_test_fn_12(){}".len() as u32)
            )]
        );
    }
}

#[test]
fn expand_derive() {
    let t = TempDir::new().unwrap();
    let project = setup_project_with_v1_and_v2_macro_deps(&t, None, None);

    let mut manifest_path = project.clone();
    manifest_path.push("test_package");
    manifest_path.set_file_name("Scarb.toml");

    let mut proc_macro_client = ProcMacroClient::new(&project);

    let component = proc_macro_client
        .defined_macros_for_package("test_package", manifest_path.clone())
        .component;

    for macro_name in ["some_derive_v1", "some_derive_v2"] {
        let code = "fn some_test_fn(){}".to_string();
        let span = TextSpan::new(0, code.len() as u32);
        let item = TokenStreamV2::new(vec![TokenTree::Ident(Token::new(code, span.clone()))]);

        let response = proc_macro_client
            .request_and_wait::<ExpandDerive>(ExpandDeriveParams {
                context: ProcMacroScope {
                    workspace: Workspace {
                        manifest_path: manifest_path.clone(),
                    },
                    component: component.clone(),
                },
                derive: macro_name.to_string(),
                item,
                call_site: span.clone(),
            })
            .unwrap();

        assert_eq!(response.diagnostics, vec![]);
        assert_eq!(
            response.token_stream.to_string(),
            "impl SomeImpl of SomeTrait {}".to_string()
        );

        let expanded = "impl SomeImpl of SomeTrait {}".to_string();
        if macro_name == "some_derive_v2" {
            // The v2 macro spans its own output.
            assert_eq!(
                tokens(&response.token_stream),
                vec![(expanded.clone(), TextSpan::new(0, expanded.len() as u32))]
            );
        } else {
            // The v1 macro reports no spans, so the whole output is attributed to the call site.
            assert_eq!(
                tokens(&response.token_stream),
                vec![(expanded.clone(), span.clone())]
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

    let mut manifest_path = project.clone();
    manifest_path.push("test_package");
    manifest_path.set_file_name("Scarb.toml");

    let mut proc_macro_client = ProcMacroClient::new(&project);

    let component = proc_macro_client
        .defined_macros_for_package("test_package", manifest_path.clone())
        .component;

    let args_code = "struct A { field: 15, other_field: macro_call!(12)}".to_string();
    let span = TextSpan::new(0, args_code.len() as u32);
    let args = TokenStreamV2::new(vec![TokenTree::Ident(Token::new(args_code, span.clone()))]);

    for macro_name in ["replace_all_15_with_25_v1", "replace_all_15_with_25_v2"] {
        let response = proc_macro_client
            .request_and_wait::<ExpandInline>(ExpandInlineMacroParams {
                context: ProcMacroScope {
                    workspace: Workspace {
                        manifest_path: manifest_path.clone(),
                    },
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

        // Both api versions report the expansion as a single token spanning the macro call.
        assert_eq!(
            tokens(&response.token_stream),
            vec![(
                "struct A { field: 25, other_field: macro_call!(12)}".to_string(),
                span.clone()
            )]
        );
    }
}

#[test]
fn v1_attribute_returning_nothing_reports_an_empty_expansion() {
    // An empty expansion asks the caller to remove the item. That has to survive the upcast from
    // the v1 api, which returns a plain string rather than a token stream.
    let remove_v1 = r#"
        #[attribute_macro]
        pub fn remove_v1(_attr: TokenStream, _token_stream: TokenStream) -> ProcMacroResult {
            ProcMacroResult::new(TokenStream::new(String::new()))
        }
    "#;

    let t = TempDir::new().unwrap();
    let project = setup_project_with_v1_and_v2_macro_deps(&t, Some(remove_v1), None);

    let mut manifest_path = project.clone();
    manifest_path.push("test_package");
    manifest_path.set_file_name("Scarb.toml");

    let mut proc_macro_client = ProcMacroClient::new(&project);
    let component = proc_macro_client
        .defined_macros_for_package("test_package", manifest_path.clone())
        .component;

    let code = "fn some_test_fn(){}".to_string();
    let span = TextSpan::new(0, code.len() as u32);
    let item = TokenStreamV2::new(vec![TokenTree::Ident(Token::new(code, span.clone()))]);

    let response = proc_macro_client
        .request_and_wait::<ExpandAttribute>(ExpandAttributeParams {
            context: ProcMacroScope {
                workspace: Workspace {
                    manifest_path: manifest_path.clone(),
                },
                component,
            },
            attr: "remove_v1".to_string(),
            args: TokenStreamV2::empty(),
            item,
            adapted_call_site: span,
        })
        .unwrap();

    assert_eq!(response.diagnostics, vec![]);
    assert!(response.token_stream.is_empty());
    assert_eq!(tokens(&response.token_stream), vec![]);
}

#[test]
fn v1_diagnostics_arrive_without_a_span() {
    // The v1 api has no way to point a diagnostic at a piece of code, so the upcast produces a
    // span-less diagnostic and the caller falls back to the macro call site.
    let failing_v1 = r#"
        use cairo_lang_macro::Diagnostic;

        #[attribute_macro]
        pub fn failing_v1(_attr: TokenStream, token_stream: TokenStream) -> ProcMacroResult {
            ProcMacroResult::new(token_stream)
                .with_diagnostics(Diagnostic::error("v1 is unhappy").into())
        }
    "#;

    let t = TempDir::new().unwrap();
    let project = setup_project_with_v1_and_v2_macro_deps(&t, Some(failing_v1), None);

    let mut manifest_path = project.clone();
    manifest_path.push("test_package");
    manifest_path.set_file_name("Scarb.toml");

    let mut proc_macro_client = ProcMacroClient::new(&project);
    let component = proc_macro_client
        .defined_macros_for_package("test_package", manifest_path.clone())
        .component;

    let code = "fn some_test_fn(){}".to_string();
    let span = TextSpan::new(0, code.len() as u32);
    let item = TokenStreamV2::new(vec![TokenTree::Ident(Token::new(code, span.clone()))]);

    let response = proc_macro_client
        .request_and_wait::<ExpandAttribute>(ExpandAttributeParams {
            context: ProcMacroScope {
                workspace: Workspace {
                    manifest_path: manifest_path.clone(),
                },
                component,
            },
            attr: "failing_v1".to_string(),
            args: TokenStreamV2::empty(),
            item,
            adapted_call_site: span,
        })
        .unwrap();

    assert_eq!(response.diagnostics.len(), 1);
    assert_eq!(response.diagnostics[0].message(), "v1 is unhappy");
    assert_eq!(response.diagnostics[0].span(), None);
}
