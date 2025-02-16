use assert_fs::fixture::PathChild;
use assert_fs::TempDir;
use indoc::indoc;
use scarb_test_support::cairo_plugin_project_builder::CairoPluginProjectBuilder;
use scarb_test_support::command::Scarb;
use scarb_test_support::project_builder::ProjectBuilder;

#[test]
fn can_return_aux_data_from_plugin() {
    let temp = TempDir::new().unwrap();
    let t = temp.child("some");
    CairoPluginProjectBuilder::default()
        .lib_rs(indoc! {r##"
        use cairo_lang_macro::{ProcMacroResult, TokenStream, attribute_macro, AuxData, PostProcessContext, post_process};
        use serde::{Serialize, Deserialize};

        #[derive(Debug, Serialize, Deserialize)]
        struct SomeMacroDataFormat {
            msg: String
        }

        #[attribute_macro]
        pub fn some(_attr: TokenStream, token_stream: TokenStream) -> ProcMacroResult {
            let value = SomeMacroDataFormat { msg: "Hello from some macro!".to_string() };
            let value = serde_json::to_string(&value).unwrap();
            let value: Vec<u8> = value.into_bytes();
            let aux_data = AuxData::new(value);
            ProcMacroResult::new(token_stream).with_aux_data(aux_data)
        }

        #[post_process]
        pub fn callback(context: PostProcessContext) {
            let aux_data = context.aux_data.into_iter()
                .map(|aux_data| {
                    let value: Vec<u8> = aux_data.into();
                    let aux_data: SomeMacroDataFormat = serde_json::from_slice(&value).unwrap();
                    aux_data
                })
                .collect::<Vec<_>>();
            println!("{:?}", aux_data);
        }

        #[post_process]
        pub fn some_no_op_callback(context: PostProcessContext) {
            drop(context.aux_data);
        }
        "##})
        .add_dep(r#"serde = { version = "*", features = ["derive"] }"#)
        .add_dep(r#"serde_json = "*""#)
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
            [SomeMacroDataFormat { msg: "Hello from some macro!" }]
            [..]Finished `dev` profile target(s) in [..]
        "#});
}

#[test]
fn can_read_token_stream_metadata() {
    let temp = TempDir::new().unwrap();
    let t = temp.child("some");
    CairoPluginProjectBuilder::default()
        .lib_rs(indoc! {r##"
        use cairo_lang_macro::{ProcMacroResult, TokenStream, attribute_macro};

        #[attribute_macro]
        pub fn some(_attr: TokenStream, token_stream: TokenStream) -> ProcMacroResult {
            println!("{:#?}", token_stream.metadata());
            ProcMacroResult::new(token_stream)
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
        .success()
        .stdout_matches(indoc! {r#"
            [..]Compiling some v1.0.0 ([..]Scarb.toml)
            [..]Compiling hello v1.0.0 ([..]Scarb.toml)
            TokenStreamMetadata {
                original_file_path: Some(
                    "[..]lib.cairo",
                ),
                file_id: Some(
                    "[..]",
                ),
                edition: Some(
                    "[..]",
                ),
            }
            [..]Finished `dev` profile target(s) in [..]
        "#});
}

#[test]
fn can_resolve_full_path_markers() {
    let temp = TempDir::new().unwrap();
    let t = temp.child("some");
    CairoPluginProjectBuilder::default()
        .lib_rs(indoc! {r##"
        use cairo_lang_macro::{ProcMacroResult, TokenStream, attribute_macro, post_process, PostProcessContext, TokenTree, Token, TextSpan};

        #[attribute_macro]
        pub fn some(_attr: TokenStream, token_stream: TokenStream) -> ProcMacroResult {
            let full_path_markers = vec!["some-key".to_string()];

            let code = format!(
                r#"#[macro::full_path_marker("some-key")] {}"#,
                token_stream.to_string().replace("12", "34")
            );

            ProcMacroResult::new(TokenStream::new(vec![TokenTree::Ident(Token::new(
              code.clone(),
                TextSpan {
                  start: 0,
                  end: code.len() as u32,
                },
              ))])
            ).with_full_path_markers(full_path_markers)
        }

        #[post_process]
        pub fn callback(context: PostProcessContext) {
            println!("{:?}", context.full_path_markers);
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
        .success()
        .stdout_matches(indoc! {r#"
            [..]Compiling some v1.0.0 ([..]Scarb.toml)
            [..]Compiling hello v1.0.0 ([..]Scarb.toml)
            [FullPathMarker { key: "some-key", full_path: "hello::main" }]
            [..]Finished `dev` profile target(s) in [..]
        "#});
}
