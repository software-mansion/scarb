use assert_fs::prelude::PathChild;
use assert_fs::TempDir;
use scarb_proc_macro_server_types::methods::defined_macros::DefinedMacros;
use scarb_proc_macro_server_types::methods::defined_macros::DefinedMacrosParams;
use scarb_test_support::cairo_plugin_project_builder::CairoPluginProjectBuilder;
use scarb_test_support::proc_macro_server::ProcMacroClient;
use scarb_test_support::proc_macro_server::SIMPLE_MACROS;
use scarb_test_support::project_builder::ProjectBuilder;

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
