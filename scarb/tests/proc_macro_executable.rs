use assert_fs::fixture::PathChild;
use assert_fs::TempDir;
use cairo_lang_sierra::program::VersionedProgram;
use indoc::indoc;
use scarb_test_support::cairo_plugin_project_builder::CairoPluginProjectBuilder;
use scarb_test_support::command::Scarb;
use scarb_test_support::fsx::ChildPathEx;
use scarb_test_support::project_builder::ProjectBuilder;

#[test]
fn can_create_executable_attribute() {
    let temp = TempDir::new().unwrap();
    let t = temp.child("some");
    CairoPluginProjectBuilder::default()
        .lib_rs(indoc! {r##"
        use cairo_lang_macro::executable_attribute;
        
        executable_attribute!("some");
        "##})
        .build(&t);

    let project = temp.child("hello");
    ProjectBuilder::start()
        .name("hello")
        .version("1.0.0")
        .dep_starknet()
        .dep("some", &t)
        .lib_cairo(indoc! {r#"
            #[some]
            fn main() -> felt252 { 12 }
        "#})
        .build(&project);

    Scarb::quick_snapbox()
        .arg("build")
        // Disable output from Cargo.
        .env("CARGO_TERM_QUIET", "true")
        .current_dir(&project)
        .assert()
        .success()
        .stdout_matches(indoc! {r#"
            [..]Compiling some v1.0.0 ([..]Scarb.toml)
            [..]Compiling hello v1.0.0 ([..]Scarb.toml)
            [..]Finished `dev` profile target(s) in [..]
        "#});
    let sierra = project
        .child("target")
        .child("dev")
        .child("hello.sierra.json")
        .read_to_string();
    let sierra = serde_json::from_str::<VersionedProgram>(&sierra).unwrap();
    let sierra = sierra.into_v1().unwrap();
    let executables = sierra.debug_info.unwrap().executables;
    assert_eq!(executables.len(), 1);
    let executables = executables.get("some").unwrap();
    assert_eq!(executables.len(), 1);
    let fid = executables.first().unwrap().clone();
    assert_eq!(fid.clone().debug_name.unwrap(), "hello::main");
    assert!(sierra
        .program
        .funcs
        .iter()
        .any(|f| f.id.clone() == fid.clone()));
}

#[test]
fn executable_name_cannot_clash_attr() {
    let temp = TempDir::new().unwrap();
    let t = temp.child("some");
    CairoPluginProjectBuilder::default()
        .lib_rs(indoc! {r##"
        use cairo_lang_macro::{executable_attribute, attribute_macro, TokenStream, ProcMacroResult};

        executable_attribute!("some");

        #[attribute_macro]
        fn some(_args: TokenStream, input: TokenStream) -> ProcMacroResult {
            ProcMacroResult::new(input)
        }
        "##})
        .build(&t);

    let project = temp.child("hello");
    ProjectBuilder::start()
        .name("hello")
        .version("1.0.0")
        .dep_starknet()
        .dep("some", &t)
        .lib_cairo(indoc! {r#"
            #[some]
            fn main() -> felt252 { 12 }
        "#})
        .build(&project);

    Scarb::quick_snapbox()
        .arg("build")
        // Disable output from Cargo.
        .env("CARGO_TERM_QUIET", "true")
        .current_dir(&project)
        .assert()
        .failure()
        .stdout_matches(indoc! {r#"
            [..]Compiling some v1.0.0 ([..]Scarb.toml)
            [..]Compiling hello v1.0.0 ([..]Scarb.toml)
            error: duplicate expansions defined for procedural macro some v1.0.0 ([..]Scarb.toml): some
        "#});
}
