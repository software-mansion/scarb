use assert_fs::prelude::PathChild;
use assert_fs::TempDir;
use cairo_lang_macro::TokenStream;
use scarb_proc_macro_server_types::methods::defined_macros::DefinedMacros;
use scarb_proc_macro_server_types::methods::defined_macros::DefinedMacrosParams;
use scarb_proc_macro_server_types::methods::expand::ExpandAttribute;
use scarb_proc_macro_server_types::methods::expand::ExpandAttributeParams;
use scarb_proc_macro_server_types::methods::expand::ExpandDerive;
use scarb_proc_macro_server_types::methods::expand::ExpandDeriveParams;
use scarb_proc_macro_server_types::methods::expand::ExpandInline;
use scarb_proc_macro_server_types::methods::expand::ExpandInlineMacroParams;
use scarb_test_support::cairo_plugin_project_builder::CairoPluginProjectBuilder;
use scarb_test_support::proc_macro_server::ProcMacroClient;
use scarb_test_support::proc_macro_server::SIMPLE_MACROS;
use scarb_test_support::project_builder::{Dep, DepBuilder, ProjectBuilder};

#[test]
fn defined_macros() {
    let t = TempDir::new().unwrap();
    let plugin_package = t.child("some");

    CairoPluginProjectBuilder::default()
        .lib_rs(SIMPLE_MACROS)
        .build(&plugin_package);

    let project = t.child("test_package");

    ProjectBuilder::start()
        .name("test_package")
        .version("1.0.0")
        .lib_cairo("")
        .dep("some", plugin_package)
        .build(&project);

    let mut proc_macro_server = ProcMacroClient::new(&project);

    let response = proc_macro_server
        .request_and_wait::<DefinedMacros>(DefinedMacrosParams {})
        .unwrap();

    assert_eq!(response.attributes, vec!["some".to_string()]);
    assert_eq!(response.derives, vec!["some_derive".to_string()]);
    assert_eq!(response.inline_macros, vec!["inline_some".to_string()]);
    assert_eq!(response.executables, vec!["some_executable".to_string()]);
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

    CairoPluginProjectBuilder::default()
        .lib_rs(format!("{SIMPLE_MACROS}\n{rename_to_very_new_name}"))
        .add_dep(r#"regex = "1.11.1""#)
        .build(&plugin_package);

    let project = t.child("test_package");

    ProjectBuilder::start()
        .name("test_package")
        .version("1.0.0")
        .lib_cairo("")
        .dep("some", plugin_package)
        .build(&project);

    let mut proc_macro_server = ProcMacroClient::new(&project);

    let response = proc_macro_server
        .request_and_wait::<ExpandAttribute>(ExpandAttributeParams {
            attr: "rename_to_very_new_name".to_string(),
            args: TokenStream::empty(),
            item: TokenStream::new("fn some_test_fn(){}".to_string()),
        })
        .unwrap();

    assert_eq!(response.diagnostics, vec![]);
    assert_eq!(
        response.token_stream,
        TokenStream::new("fn very_new_name(){}".to_string())
    );
}

#[test]
fn expand_derive() {
    let t = TempDir::new().unwrap();
    let plugin_package = t.child("some");

    CairoPluginProjectBuilder::default()
        .lib_rs(SIMPLE_MACROS)
        .build(&plugin_package);

    let project = t.child("test_package");

    ProjectBuilder::start()
        .name("test_package")
        .version("1.0.0")
        .lib_cairo("")
        .dep("some", plugin_package)
        .build(&project);

    let mut proc_macro_server = ProcMacroClient::new(&project);

    let item = TokenStream::new("fn some_test_fn(){}".to_string());

    let response = proc_macro_server
        .request_and_wait::<ExpandDerive>(ExpandDeriveParams {
            derives: vec!["some_derive".to_string()],
            item,
        })
        .unwrap();

    assert_eq!(response.diagnostics, vec![]);
    assert_eq!(
        response.token_stream,
        TokenStream::new("impl SomeImpl of SomeTrait {}".to_string())
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

    CairoPluginProjectBuilder::default()
        .lib_rs(format!("{SIMPLE_MACROS}\n{replace_all_15_with_25}"))
        .build(&plugin_package);

    let project = t.child("test_package");

    ProjectBuilder::start()
        .name("test_package")
        .version("1.0.0")
        .lib_cairo("")
        .dep("some", plugin_package)
        .build(&project);

    let mut proc_macro_server = ProcMacroClient::new(&project);

    let response = proc_macro_server
        .request_and_wait::<ExpandInline>(ExpandInlineMacroParams {
            name: "replace_all_15_with_25".to_string(),
            args: TokenStream::new(
                "struct A { field: 15 , other_field: macro_call!(12)}".to_string(),
            ),
        })
        .unwrap();

    assert_eq!(response.diagnostics, vec![]);
    assert_eq!(
        response.token_stream,
        TokenStream::new("struct A { field: 25 , other_field: macro_call!(12)}".to_string())
    );
}

#[test]
fn load_prebuilt_proc_macros() {
    let t = TempDir::new().unwrap();

    let project = t.child("test_package");

    ProjectBuilder::start()
        .name("test_package")
        .version("1.0.0")
        .lib_cairo("")
        .dep(
            "proc_macro_example",
            Dep.version("0.1.2").registry("https://scarbs.dev/"),
        )
        .build(&project);

    let mut proc_macro_server = ProcMacroClient::new_without_cargo(&project);

    let response = proc_macro_server
        .request_and_wait::<ExpandInline>(ExpandInlineMacroParams {
            name: "some".to_string(),
            args: TokenStream::new("42".to_string()),
        })
        .unwrap();

    assert_eq!(response.diagnostics, vec![]);
    assert_eq!(response.token_stream, TokenStream::new("42".to_string()));
}
