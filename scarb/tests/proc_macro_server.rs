use assert_fs::prelude::PathChild;
use assert_fs::TempDir;
use cairo_lang_macro::TokenStream;
use scarb_proc_macro_server_types::context::RequestContext;
use scarb_proc_macro_server_types::methods::defined_macros::ComponentDefinedMacrosInfo;
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
use scarb_test_support::project_builder::ProjectBuilder;

/// A helper structure to store the relate the PMS response to the ID of the main compilation unit's member.
struct DefinedMacrosInfo {
    /// ID of the main compilation unit's member, recognized via PMS.
    compilation_unit_member_id: String,
    /// A proper part of the response, related to the main component of the main CU.
    component_macros: ComponentDefinedMacrosInfo,
}

/// Returns the information about macros available for CU component
/// of the main compilation unit associated with package of `package_name`.
/// Used as a helper in PMS tests, where concrete IDs assigned by Scarb are required.
fn request_proc_macros_for_member_package(
    client: &mut ProcMacroClient,
    package_name: &str,
) -> DefinedMacrosInfo {
    let response = client
        .request_and_wait::<DefinedMacros>(DefinedMacrosParams {})
        .unwrap();

    let mut response = response.workspace_macro_info;

    let compilation_unit_member_id = response
        .keys()
        .find(|cu_id| cu_id.starts_with(package_name))
        .expect("Response from Proc Macro Server should contain the main compilation unit.")
        .to_owned();

    let component_macros = response
        .remove(&compilation_unit_member_id)
        .unwrap()
        .remove(&compilation_unit_member_id) // We are analyzing the main component of the CU.
        .expect(
            "Response from Proc Macro Server should contain the main compilation unit component.",
        );

    DefinedMacrosInfo {
        compilation_unit_member_id,
        component_macros,
    }
}

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

    let mut proc_macro_client = ProcMacroClient::new(&project);

    let DefinedMacrosInfo {
        component_macros, ..
    } = request_proc_macros_for_member_package(&mut proc_macro_client, "test_package");

    assert_eq!(&component_macros.attributes, &["some".to_string()]);
    assert_eq!(&component_macros.derives, &["some_derive".to_string()]);
    assert_eq!(
        &component_macros.inline_macros,
        &["inline_some".to_string()]
    );
    assert_eq!(
        &component_macros.executables,
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

    let mut proc_macro_client = ProcMacroClient::new(&project);

    let DefinedMacrosInfo {
        compilation_unit_member_id,
        ..
    } = request_proc_macros_for_member_package(&mut proc_macro_client, "test_package");

    let response = proc_macro_client
        .request_and_wait::<ExpandAttribute>(ExpandAttributeParams {
            context: RequestContext {
                compilation_unit_id: compilation_unit_member_id.clone(),
                compilation_unit_component_id: compilation_unit_member_id,
            },
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

    let mut proc_macro_client = ProcMacroClient::new(&project);

    let DefinedMacrosInfo {
        compilation_unit_member_id,
        ..
    } = request_proc_macros_for_member_package(&mut proc_macro_client, "test_package");

    let item = TokenStream::new("fn some_test_fn(){}".to_string());

    let response = proc_macro_client
        .request_and_wait::<ExpandDerive>(ExpandDeriveParams {
            context: RequestContext {
                compilation_unit_id: compilation_unit_member_id.clone(),
                compilation_unit_component_id: compilation_unit_member_id,
            },
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

    let mut proc_macro_client = ProcMacroClient::new(&project);

    let DefinedMacrosInfo {
        compilation_unit_member_id,
        ..
    } = request_proc_macros_for_member_package(&mut proc_macro_client, "test_package");

    let response = proc_macro_client
        .request_and_wait::<ExpandInline>(ExpandInlineMacroParams {
            context: RequestContext {
                compilation_unit_id: compilation_unit_member_id.clone(),
                compilation_unit_component_id: compilation_unit_member_id,
            },
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
