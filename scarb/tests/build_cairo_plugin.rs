use assert_fs::fixture::PathChild;
use assert_fs::TempDir;
use cairo_lang_sierra::program::VersionedProgram;
use indoc::indoc;
use scarb_test_support::cairo_plugin_project_builder::CairoPluginProjectBuilder;
use scarb_test_support::command::Scarb;
use scarb_test_support::fsx::ChildPathEx;
use scarb_test_support::project_builder::ProjectBuilder;
use snapbox::assert_matches;

#[test]
fn compile_cairo_plugin() {
    let t = TempDir::new().unwrap();
    CairoPluginProjectBuilder::default().build(&t);
    let output = Scarb::quick_snapbox()
        .arg("build")
        // Disable colors in Cargo output.
        .env("CARGO_TERM_COLOR", "never")
        .current_dir(&t)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "stdout={}\n stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    assert!(stdout.contains("Compiling some v1.0.0"));
    let lines = stdout.lines().map(ToString::to_string).collect::<Vec<_>>();
    let (last, lines) = lines.split_last().unwrap();
    assert_matches(r#"[..] Finished `dev` profile target(s) in [..]"#, last);
    let (last, _lines) = lines.split_last().unwrap();
    // Line from Cargo output
    assert_matches(
        r#"[..]Finished `release` profile [optimized] target(s) in[..]"#,
        last,
    );
}

#[test]
fn check_cairo_plugin() {
    let t = TempDir::new().unwrap();
    CairoPluginProjectBuilder::default().build(&t);
    let output = Scarb::quick_snapbox()
        .arg("check")
        // Disable colors in Cargo output.
        .env("CARGO_TERM_COLOR", "never")
        .current_dir(&t)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    assert!(stdout.contains("Checking some v1.0.0"));
    let lines = stdout.lines().map(ToString::to_string).collect::<Vec<_>>();
    let (last, lines) = lines.split_last().unwrap();
    assert_matches(
        r#"[..] Finished checking `dev` profile target(s) in [..]"#,
        last,
    );
    let (last, _lines) = lines.split_last().unwrap();
    // Line from Cargo output
    assert_matches(
        r#"[..]Finished `release` profile [optimized] target(s) in[..]"#,
        last,
    );
}

#[test]
fn resolve_fetched_plugins() {
    let t = TempDir::new().unwrap();
    CairoPluginProjectBuilder::default().build(&t);
    assert!(!t.child("Cargo.lock").exists());
    let output = Scarb::quick_snapbox()
        .arg("fetch")
        // Disable colors in Cargo output.
        .env("CARGO_TERM_COLOR", "never")
        .current_dir(&t)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(t.child("Cargo.lock").exists())
}

#[test]
fn can_use_json_output() {
    let t = TempDir::new().unwrap();
    CairoPluginProjectBuilder::default().build(&t);
    let output = Scarb::quick_snapbox()
        .arg("--json")
        .arg("check")
        // Disable colors in Cargo output.
        .env("CARGO_TERM_COLOR", "never")
        .current_dir(&t)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let lines = stdout.lines().map(ToString::to_string).collect::<Vec<_>>();
    let (first, lines) = lines.split_first().unwrap();
    assert_matches(
        r#"{"status":"checking","message":"some v1.0.0 ([..]Scarb.toml)"}"#,
        first,
    );
    let (last, lines) = lines.split_last().unwrap();
    assert_matches(
        r#"{"status":"finished","message":"checking `dev` profile target(s) in [..]"}"#,
        last,
    );
    // Line from Cargo.
    let (last, _lines) = lines.split_last().unwrap();
    assert_matches(r#"{"reason":"build-finished","success":true}"#, last);
}

#[test]
fn compile_cairo_plugin_with_lib_target() {
    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello")
        .version("1.0.0")
        .manifest_extra(indoc! {r#"
            [lib]
            [cairo-plugin]
        "#})
        .build(&t);

    Scarb::quick_snapbox()
        .arg("build")
        .current_dir(&t)
        .assert()
        .failure()
        .stdout_matches(indoc! {r#"
        error: failed to parse manifest at: [..]/Scarb.toml

        Caused by:
            target `cairo-plugin` cannot be mixed with other targets
        "#});
}

#[test]
fn compile_cairo_plugin_with_other_target() {
    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello")
        .version("1.0.0")
        .manifest_extra(indoc! {r#"
            [cairo-plugin]
            [[target.starknet-contract]]
        "#})
        .build(&t);

    Scarb::quick_snapbox()
        .arg("build")
        .current_dir(&t)
        .assert()
        .failure()
        .stdout_matches(indoc! {r#"
        error: failed to parse manifest at: [..]/Scarb.toml

        Caused by:
            target `cairo-plugin` cannot be mixed with other targets
        "#});
}

#[test]
fn can_emit_plugin_warning() {
    let temp = TempDir::new().unwrap();
    let t = temp.child("some");
    CairoPluginProjectBuilder::default()
        .lib_rs(indoc! {r#"
        use cairo_lang_macro::{ProcMacroResult, TokenStream, attribute_macro, Diagnostic};

        #[attribute_macro]
        pub fn some(_attr: TokenStream, token_stream: TokenStream) -> ProcMacroResult {
            let diag = Diagnostic::warn("Some warning from macro.");
            ProcMacroResult::new(token_stream)
                .with_diagnostics(diag.into())
        }
        "#})
        .build(&t);
    let project = temp.child("hello");
    ProjectBuilder::start()
        .name("hello")
        .version("1.0.0")
        .dep("some", &t)
        .lib_cairo(indoc! {r#"
            #[some]
            fn f() -> felt252 { 12 }
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
            [..] Compiling some v1.0.0 ([..]Scarb.toml)
            [..] Compiling hello v1.0.0 ([..]Scarb.toml)
            warn: Plugin diagnostic: Some warning from macro.
             --> [..]lib.cairo:1:1
            #[some]
            ^*****^

            [..]Finished `dev` profile target(s) in [..]
        "#});
}

#[test]
fn can_emit_plugin_error() {
    let temp = TempDir::new().unwrap();
    let t = temp.child("some");
    CairoPluginProjectBuilder::default()
        .lib_rs(indoc! {r#"
        use cairo_lang_macro::{ProcMacroResult, TokenStream, attribute_macro, Diagnostic};

        #[attribute_macro]
        pub fn some(_attr: TokenStream, token_stream: TokenStream) -> ProcMacroResult {
            let diag = Diagnostic::error("Some error from macro.");
            ProcMacroResult::new(token_stream)
                .with_diagnostics(diag.into())
        }
        "#})
        .build(&t);
    let project = temp.child("hello");
    ProjectBuilder::start()
        .name("hello")
        .version("1.0.0")
        .dep("some", &t)
        .lib_cairo(indoc! {r#"
            #[some]
            fn f() -> felt252 { 12 }
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
            [..] Compiling some v1.0.0 ([..]Scarb.toml)
            [..] Compiling hello v1.0.0 ([..]Scarb.toml)
            error: Plugin diagnostic: Some error from macro.
             --> [..]lib.cairo:1:1
            #[some]
            ^*****^

            error: could not compile `hello` due to previous error
        "#});
}

#[test]
fn diags_from_generated_code_mapped_correctly() {
    let temp = TempDir::new().unwrap();
    let t = temp.child("some");
    CairoPluginProjectBuilder::default()
        .lib_rs(indoc! {r#"
        use cairo_lang_macro::{ProcMacroResult, TokenStream, attribute_macro, Diagnostic};

        #[attribute_macro]
        pub fn some(_attr: TokenStream, token_stream: TokenStream) -> ProcMacroResult {
            let diag = Diagnostic::error("Some error from macro.");
            ProcMacroResult::new(token_stream)
                 .with_diagnostics(diag.into())
        }
        "#})
        .build(&t);
    let project = temp.child("hello");
    ProjectBuilder::start()
        .name("hello")
        .version("1.0.0")
        .dep("some", &t)
        .lib_cairo(indoc! {r#"
        #[cfg(target: 'lib')]
        #[some]
        fn test_increase_balance() {
            i_don_exist();
        }
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
            [..] Compiling some v1.0.0 ([..]Scarb.toml)
            [..] Compiling hello v1.0.0 ([..]Scarb.toml)
            error: Plugin diagnostic: Some error from macro.
             --> [..]lib.cairo:2:1
            #[some]
            ^*****^
            
            error: Function not found.
             --> [..]lib.cairo:4:5
                i_don_exist();
                ^*********^
            
            error: could not compile `hello` due to previous error
    "#});
}

#[test]
fn can_remove_original_node() {
    let temp = TempDir::new().unwrap();
    let t = temp.child("some");
    CairoPluginProjectBuilder::default()
        .lib_rs(indoc! {r#"
        use cairo_lang_macro::{ProcMacroResult, TokenStream, attribute_macro};

        #[attribute_macro]
        pub fn some(_attr: TokenStream, _: TokenStream) -> ProcMacroResult {
            ProcMacroResult::new(TokenStream::empty())
        }
        "#})
        .build(&t);
    let project = temp.child("hello");
    ProjectBuilder::start()
        .name("hello")
        .version("1.0.0")
        .dep("some", &t)
        .lib_cairo(indoc! {r#"
            #[some]
            fn main() -> felt252 { 12 }

            fn main() -> felt252 { 34 }

            #[some]
            fn main() -> felt252 { 56 }
        "#})
        .build(&project);

    Scarb::quick_snapbox()
        .arg("cairo-run")
        // Disable output from Cargo.
        .env("CARGO_TERM_QUIET", "true")
        .current_dir(&project)
        .assert()
        .success()
        .stdout_matches(indoc! {r#"
            [..] Compiling some v1.0.0 ([..]Scarb.toml)
            [..] Compiling hello v1.0.0 ([..]Scarb.toml)
            [..]Finished `dev` profile target(s) in [..]
            [..]Running hello
            Run completed successfully, returning [34]
        "#});
}

#[test]
fn can_replace_original_node() {
    let temp = TempDir::new().unwrap();
    let t = temp.child("some");
    CairoPluginProjectBuilder::default()
        .lib_rs(indoc! {r##"
        use cairo_lang_macro::{ProcMacroResult, TokenStream, attribute_macro};

        #[attribute_macro]
        pub fn some(_attr: TokenStream, token_stream: TokenStream) -> ProcMacroResult {
            let token_stream = TokenStream::new(
                token_stream
                    .to_string()
                    .replace("12", "34")
            );
            ProcMacroResult::new(token_stream)
        }
        "##})
        .build(&t);
    let project = temp.child("hello");
    ProjectBuilder::start()
        .name("hello")
        .version("1.0.0")
        .dep("some", &t)
        .lib_cairo(indoc! {r#"
            #[some]
            fn main() -> felt252 { 12 }
        "#})
        .build(&project);

    Scarb::quick_snapbox()
        .arg("cairo-run")
        // Disable output from Cargo.
        .env("CARGO_TERM_QUIET", "true")
        .current_dir(&project)
        .assert()
        .success()
        .stdout_matches(indoc! {r#"
            [..] Compiling some v1.0.0 ([..]Scarb.toml)
            [..] Compiling hello v1.0.0 ([..]Scarb.toml)
            [..]Finished `dev` profile target(s) in [..]
            [..]Running hello
            Run completed successfully, returning [34]
        "#});
}

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
            }
            [..]Finished `dev` profile target(s) in [..]
        "#});
}

#[test]
fn can_define_multiple_macros() {
    let temp = TempDir::new().unwrap();
    let t = temp.child("some");
    CairoPluginProjectBuilder::default()
        .lib_rs(indoc! {r##"
        use cairo_lang_macro::{ProcMacroResult, TokenStream, attribute_macro, AuxData, PostProcessContext, post_process};

        #[attribute_macro]
        pub fn hello(_attr: TokenStream, token_stream: TokenStream) -> ProcMacroResult {
            let token_stream = TokenStream::new(
                token_stream
                    .to_string()
                    .replace("12", "34")
            );
            let aux_data = AuxData::new(Vec::new());
            ProcMacroResult::new(token_stream).with_aux_data(aux_data)
        }

        #[attribute_macro]
        pub fn world(_attr: TokenStream, token_stream: TokenStream) -> ProcMacroResult {
            let token_stream = TokenStream::new(
                token_stream
                    .to_string()
                    .replace("56", "78")
            );
            let aux_data = AuxData::new(Vec::new());
            ProcMacroResult::new(token_stream).with_aux_data(aux_data)
        }

        #[post_process]
        pub fn callback(context: PostProcessContext) {
            assert_eq!(context.aux_data.len(), 2);
        }
        "##})
        .build(&t);

    let w = temp.child("other");
    CairoPluginProjectBuilder::default()
        .name("other")
        .lib_rs(indoc! {r##"
        use cairo_lang_macro::{ProcMacroResult, TokenStream, attribute_macro, AuxData, PostProcessContext, post_process};

        #[attribute_macro]
        pub fn beautiful(_attr: TokenStream, token_stream: TokenStream) -> ProcMacroResult {
            let token_stream = TokenStream::new(
                token_stream
                    .to_string()
                    .replace("90", "09")
            );
            let aux_data = AuxData::new(Vec::new());
            ProcMacroResult::new(token_stream).with_aux_data(aux_data)
        }

        #[post_process]
        pub fn callback(context: PostProcessContext) {
            assert_eq!(context.aux_data.len(), 1);
        }
        "##})
        .build(&w);

    let project = temp.child("hello");
    ProjectBuilder::start()
        .name("hello")
        .version("1.0.0")
        .dep_starknet()
        .dep("some", &t)
        .dep("other", &w)
        .lib_cairo(indoc! {r#"
            #[hello]
            #[beautiful]
            #[world]
            fn main() -> felt252 { 12 + 56 + 90 }
        "#})
        .build(&project);

    Scarb::quick_snapbox()
        .arg("cairo-run")
        // Disable output from Cargo.
        .env("CARGO_TERM_QUIET", "true")
        .current_dir(&project)
        .assert()
        .success()
        .stdout_matches(indoc! {r#"
            [..]Compiling some v1.0.0 ([..]Scarb.toml)
            [..]Compiling other v1.0.0 ([..]Scarb.toml)
            [..]Compiling hello v1.0.0 ([..]Scarb.toml)
            [..]Finished `dev` profile target(s) in [..]
            [..]Running hello
            Run completed successfully, returning [121]
        "#});
}

#[test]
fn cannot_duplicate_macros() {
    let temp = TempDir::new().unwrap();
    let t = temp.child("some");
    CairoPluginProjectBuilder::default()
        .lib_rs(indoc! {r##"
        use cairo_lang_macro::{ProcMacroResult, TokenStream, attribute_macro};

        #[attribute_macro]
        pub fn hello(_attr: TokenStream, token_stream: TokenStream) -> ProcMacroResult {
            ProcMacroResult::new(token_stream)
        }

        #[attribute_macro]
        pub fn hello(_attr: TokenStream, token_stream: TokenStream) -> ProcMacroResult {
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
            #[hello]
            fn main() -> felt252 { 12 + 56 + 90 }
        "#})
        .build(&project);
    Scarb::quick_snapbox()
        .arg("build")
        // Disable output from Cargo.
        .env("CARGO_TERM_QUIET", "true")
        .current_dir(&project)
        .assert()
        // Fails with Cargo compile error.
        .failure();
}

#[test]
fn cannot_duplicate_macros_across_packages() {
    let temp = TempDir::new().unwrap();
    let t = temp.child("some");
    CairoPluginProjectBuilder::default()
        .lib_rs(indoc! {r#"
        use cairo_lang_macro::{ProcMacroResult, TokenStream, attribute_macro};

        #[attribute_macro]
        pub fn hello(_attr: TokenStream, token_stream: TokenStream) -> ProcMacroResult {
            ProcMacroResult::new(token_stream)
        }

        #[attribute_macro]
        pub fn world(_attr: TokenStream, token_stream: TokenStream) -> ProcMacroResult {
            ProcMacroResult::new(token_stream)
        }
        "#})
        .build(&t);

    let w = temp.child("other");
    CairoPluginProjectBuilder::default()
        .name("other")
        .lib_rs(indoc! {r#"
        use cairo_lang_macro::{ProcMacroResult, TokenStream, attribute_macro};

        #[attribute_macro]
        pub fn hello(_attr: TokenStream, token_stream: TokenStream) -> ProcMacroResult {
            ProcMacroResult::new(token_stream)
        }
        "#})
        .build(&w);

    let project = temp.child("hello");
    ProjectBuilder::start()
        .name("hello")
        .version("1.0.0")
        .dep_starknet()
        .dep("some", &t)
        .dep("other", &w)
        .lib_cairo(indoc! {r#"
            #[hello]
            #[world]
            fn main() -> felt252 { 12 + 56 + 90 }
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
            [..]Compiling other v1.0.0 ([..]Scarb.toml)
            [..]Compiling hello v1.0.0 ([..]Scarb.toml)
            error: duplicate expansions defined for procedural macros: hello (some v1.0.0 ([..]Scarb.toml) and other v1.0.0 ([..]Scarb.toml))
        "#});
}

#[test]
fn cannot_use_undefined_macro() {
    let temp = TempDir::new().unwrap();
    let t = temp.child("some");
    CairoPluginProjectBuilder::default()
        .lib_rs(indoc! {r##"
        use cairo_lang_macro::{ProcMacroResult, TokenStream, attribute_macro};

        #[attribute_macro]
        pub fn hello(_attr: TokenStream, token_stream: TokenStream) -> ProcMacroResult {
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
            #[world]
            fn main() -> felt252 { 12 + 56 + 90 }
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
        error: Plugin diagnostic: Unsupported attribute.
         --> [..]lib.cairo:1:1
        #[world]
        ^******^

        error: could not compile `hello` due to previous error
        "#});
}

#[test]
fn can_resolve_full_path_markers() {
    let temp = TempDir::new().unwrap();
    let t = temp.child("some");
    CairoPluginProjectBuilder::default()
        .lib_rs(indoc! {r##"
        use cairo_lang_macro::{ProcMacroResult, TokenStream, attribute_macro, post_process, PostProcessContext};

        #[attribute_macro]
        pub fn some(_attr: TokenStream, token_stream: TokenStream) -> ProcMacroResult {
            let full_path_markers = vec!["some-key".to_string()];

            let code = format!(
                r#"#[macro::full_path_marker("some-key")] {}"#,
                token_stream.to_string().replace("12", "34")
            );

            ProcMacroResult::new(TokenStream::new(code))
                .with_full_path_markers(full_path_markers)
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

#[test]
fn can_implement_inline_macro() {
    let temp = TempDir::new().unwrap();
    let t = temp.child("some");
    CairoPluginProjectBuilder::default()
        .lib_rs(indoc! {r##"
        use cairo_lang_macro::{ProcMacroResult, TokenStream, inline_macro};

        #[inline_macro]
        pub fn some(_token_stream: TokenStream) -> ProcMacroResult {
            ProcMacroResult::new(TokenStream::new("34".to_string()))
        }
        "##})
        .build(&t);
    let project = temp.child("hello");
    ProjectBuilder::start()
        .name("hello")
        .version("1.0.0")
        .dep("some", &t)
        .lib_cairo(indoc! {r#"
            fn main() -> felt252 { some!() }
        "#})
        .build(&project);

    Scarb::quick_snapbox()
        .arg("cairo-run")
        // Disable output from Cargo.
        .env("CARGO_TERM_QUIET", "true")
        .current_dir(&project)
        .assert()
        .success()
        .stdout_matches(indoc! {r#"
            [..] Compiling some v1.0.0 ([..]Scarb.toml)
            [..] Compiling hello v1.0.0 ([..]Scarb.toml)
            [..]Finished `dev` profile target(s) in [..]
            [..]Running hello
            Run completed successfully, returning [34]
        "#});
}

#[test]
fn empty_inline_macro_result() {
    let temp = TempDir::new().unwrap();
    let t = temp.child("some");
    CairoPluginProjectBuilder::default()
        .lib_rs(indoc! {r##"
        use cairo_lang_macro::{ProcMacroResult, TokenStream, inline_macro};

        #[inline_macro]
        pub fn some(_token_stream: TokenStream) -> ProcMacroResult {
            ProcMacroResult::new(TokenStream::empty())
        }
        "##})
        .build(&t);
    let project = temp.child("hello");
    ProjectBuilder::start()
        .name("hello")
        .version("1.0.0")
        .dep("some", &t)
        .lib_cairo(indoc! {r#"
            fn main() -> felt252 {
                let _x = some!();
                12
            }
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
            [..] Compiling some v1.0.0 ([..]Scarb.toml)
            [..] Compiling hello v1.0.0 ([..]Scarb.toml)
            error: Inline macro `some` failed.
             --> [..]lib.cairo:2:14
                let _x = some!();
                         ^*****^
            
            error: could not compile `hello` due to previous error
        "#});
}

#[test]
fn can_implement_derive_macro() {
    let temp = TempDir::new().unwrap();
    let t = temp.child("some");
    CairoPluginProjectBuilder::default()
        .lib_rs(indoc! {r##"
            use cairo_lang_macro::{derive_macro, ProcMacroResult, TokenStream};

            #[derive_macro]
            pub fn custom_derive(token_stream: TokenStream) -> ProcMacroResult {
                let name = token_stream
                    .clone()
                    .to_string()
                    .lines()
                    .find(|l| l.starts_with("struct"))
                    .unwrap()
                    .to_string()
                    .replace("struct", "")
                    .replace("}", "")
                    .replace("{", "")
                    .trim()
                    .to_string();

                let token_stream = TokenStream::new(indoc::formatdoc!{r#"
                    impl SomeImpl of Hello<{name}> {{
                        fn world(self: @{name}) -> u32 {{
                            32
                        }}
                    }}
                "#});

                ProcMacroResult::new(token_stream)
            }
        "##})
        .add_dep(r#"indoc = "*""#)
        .build(&t);

    let project = temp.child("hello");
    ProjectBuilder::start()
        .name("hello")
        .version("1.0.0")
        .dep("some", &t)
        .lib_cairo(indoc! {r#"
            trait Hello<T> {
                fn world(self: @T) -> u32;
            }

            #[derive(CustomDerive, Drop)]
            struct SomeType {}

            fn main() -> u32 {
                let a = SomeType {};
                a.world()
            }
        "#})
        .build(&project);

    Scarb::quick_snapbox()
        .arg("cairo-run")
        // Disable output from Cargo.
        .env("CARGO_TERM_QUIET", "true")
        .current_dir(&project)
        .assert()
        .success()
        .stdout_matches(indoc! {r#"
            [..] Compiling some v1.0.0 ([..]Scarb.toml)
            [..] Compiling hello v1.0.0 ([..]Scarb.toml)
            [..]Finished `dev` profile target(s) in [..]
            [..]Running hello
            Run completed successfully, returning [32]
        "#});
}

#[test]
fn can_use_both_derive_and_attr() {
    let temp = TempDir::new().unwrap();
    let t = temp.child("some");
    CairoPluginProjectBuilder::default()
        .lib_rs(indoc! {r##"
            use cairo_lang_macro::{derive_macro, attribute_macro, ProcMacroResult, TokenStream};

            #[attribute_macro]
            pub fn first_attribute(_attr: TokenStream, token_stream: TokenStream) -> ProcMacroResult {
                ProcMacroResult::new(TokenStream::new(
                    token_stream.to_string()
                    .replace("SomeType", "OtherType")
                ))
            }

            #[attribute_macro]
            pub fn second_attribute(_attr: TokenStream, token_stream: TokenStream) -> ProcMacroResult {
                let token_stream = TokenStream::new(
                    token_stream.to_string().replace("OtherType", "RenamedStruct")
                );
                ProcMacroResult::new(TokenStream::new(
                    format!("#[derive(Drop)]\n{token_stream}")
                ))
            }

            #[derive_macro]
            pub fn custom_derive(_token_stream: TokenStream) -> ProcMacroResult {
                ProcMacroResult::new(TokenStream::new(
                    indoc::formatdoc!{r#"
                    impl SomeImpl of Hello<RenamedStruct> {{
                        fn world(self: @RenamedStruct) -> u32 {{
                            32
                        }}
                    }}
                    "#}
                ))
            }
        "##})
        .add_dep(r#"indoc = "*""#)
        .build(&t);

    let project = temp.child("hello");
    ProjectBuilder::start()
        .name("hello")
        .version("1.0.0")
        .dep("some", &t)
        .lib_cairo(indoc! {r#"
            trait Hello<T> {
                fn world(self: @T) -> u32;
            }

            #[first_attribute]
            #[derive(CustomDerive)]
            #[second_attribute]
            struct SomeType {}

            fn main() -> u32 {
                let a = RenamedStruct {};
                a.world()
            }
        "#})
        .build(&project);

    Scarb::quick_snapbox()
        .arg("cairo-run")
        // Disable output from Cargo.
        .env("CARGO_TERM_QUIET", "true")
        .current_dir(&project)
        .assert()
        .success()
        .stdout_matches(indoc! {r#"
            [..] Compiling some v1.0.0 ([..]Scarb.toml)
            [..] Compiling hello v1.0.0 ([..]Scarb.toml)
            [..]Finished `dev` profile target(s) in [..]
            [..]Running hello
            Run completed successfully, returning [32]
        "#});
}

#[test]
fn can_read_attribute_args() {
    let temp = TempDir::new().unwrap();
    let t = temp.child("some");
    CairoPluginProjectBuilder::default()
        .lib_rs(indoc! {r##"
        use cairo_lang_macro::{ProcMacroResult, TokenStream, attribute_macro};

        #[attribute_macro]
        pub fn some(attr: TokenStream, token_stream: TokenStream) -> ProcMacroResult {
            println!("{}", attr);
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
            #[some(
                first: "aaa",
                second: "bbb",
            )]
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
            (
                first: "aaa",
                second: "bbb",
            )
            [..]Finished `dev` profile target(s) in [..]
        "#});
}

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

#[test]
fn can_be_expanded() {
    let temp = TempDir::new().unwrap();
    let t = temp.child("some");
    CairoPluginProjectBuilder::default()
        .lib_rs(indoc! {r##"
        use cairo_lang_macro::{ProcMacroResult, TokenStream, attribute_macro, derive_macro};

        #[attribute_macro]
        pub fn some(_attr: TokenStream, token_stream: TokenStream) -> ProcMacroResult {
            let token_stream = TokenStream::new(
                token_stream
                    .to_string()
                    .replace("12", "34")
            );
            ProcMacroResult::new(token_stream)
        }

        #[derive_macro]
        pub fn custom_derive(token_stream: TokenStream) -> ProcMacroResult {
            let name = token_stream
                .clone()
                .to_string()
                .lines()
                .find(|l| l.starts_with("struct"))
                .unwrap()
                .to_string()
                .replace("struct", "")
                .replace("}", "")
                .replace("{", "")
                .trim()
                .to_string();

            let token_stream = TokenStream::new(indoc::formatdoc!{r#"
                impl SomeImpl of Hello<{name}> {{
                    fn world(self: @{name}) -> u32 {{
                        32
                    }}
                }}
            "#});

            ProcMacroResult::new(token_stream)
        }
        "##})
        .add_dep(r#"indoc = "*""#)
        .build(&t);
    let project = temp.child("hello");
    ProjectBuilder::start()
        .name("hello")
        .version("1.0.0")
        .dep("some", &t)
        .lib_cairo(indoc! {r#"
            trait Hello<T> {
                fn world(self: @T) -> u32;
            }

            #[derive(CustomDerive, Drop)]
            struct SomeType {}

            #[some]
            fn main() -> u32 {
                let x = 12;
                let a = SomeType {};
                a.world() + x
            }
        "#})
        .build(&project);

    Scarb::quick_snapbox()
        .arg("expand")
        // Disable output from Cargo.
        .env("CARGO_TERM_QUIET", "true")
        .current_dir(&project)
        .assert()
        .success();

    assert_eq!(
        project.child("target/dev").files(),
        vec!["hello.expanded.cairo"]
    );
    let expanded = project
        .child("target/dev/hello.expanded.cairo")
        .read_to_string();
    snapbox::assert_eq(
        indoc! {r#"
        mod hello {
            trait Hello<T> {
                fn world(self: @T) -> u32;
            }

            #[derive(CustomDerive, Drop)]
            struct SomeType {}
            impl SomeTypeDrop of core::traits::Drop<SomeType>;
            impl SomeImpl of Hello<SomeType> {
                fn world(self: @SomeType) -> u32 {
                    32
                }
            }
            fn main() -> u32 {
                let x = 34;
                let a = SomeType {};
                a.world() + x
            }
        }
        "#},
        expanded,
    );
}

#[test]
fn can_expand_trait_inner_func_attrr() {
    let temp = TempDir::new().unwrap();
    let t = temp.child("some");
    CairoPluginProjectBuilder::default()
        .lib_rs(indoc! {r##"
            use cairo_lang_macro::{attribute_macro, ProcMacroResult, TokenStream};

            #[attribute_macro]
            pub fn some(_attr: TokenStream, token_stream: TokenStream) -> ProcMacroResult {
                ProcMacroResult::new(TokenStream::new(
                    token_stream.to_string()
                    .replace("hello", "world")
                    .replace("12", "34")
                ))
            }
        "##})
        .build(&t);

    let project = temp.child("hello");
    ProjectBuilder::start()
        .name("hello")
        .version("1.0.0")
        .dep("some", &t)
        .lib_cairo(indoc! {r#"
            trait Hello<T> {
                #[some]
                fn hello(self: @T) -> u32 {
                    12
                }
            }

            #[derive(Drop)]
            struct SomeStruct {}

            impl SomeImpl of Hello<SomeStruct> {}

            fn main() -> u32 {
                let a = SomeStruct {};
                a.world()
            }
        "#})
        .build(&project);

    Scarb::quick_snapbox()
        .arg("cairo-run")
        // Disable output from Cargo.
        .env("CARGO_TERM_QUIET", "true")
        .current_dir(&project)
        .assert()
        .success()
        .stdout_matches(indoc! {r#"
            [..] Compiling some v1.0.0 ([..]Scarb.toml)
            [..] Compiling hello v1.0.0 ([..]Scarb.toml)
            [..]Finished `dev` profile target(s) in [..]
            [..]Running hello
            Run completed successfully, returning [34]
        "#});
}

#[test]
fn can_expand_impl_inner_func_attrr() {
    let temp = TempDir::new().unwrap();
    let t = temp.child("some");
    CairoPluginProjectBuilder::default()
        .lib_rs(indoc! {r##"
            use cairo_lang_macro::{attribute_macro, ProcMacroResult, TokenStream};

            #[attribute_macro]
            pub fn some(_attr: TokenStream, token_stream: TokenStream) -> ProcMacroResult {
                ProcMacroResult::new(TokenStream::new(
                    token_stream.to_string()
                    .replace("1", "2")
                ))
            }
        "##})
        .build(&t);

    let project = temp.child("hello");
    ProjectBuilder::start()
        .name("hello")
        .version("1.0.0")
        .dep_starknet()
        .dep_cairo_test()
        .dep("some", &t)
        .manifest_extra(indoc! {r#"
            [[target.starknet-contract]]
        "#})
        .lib_cairo(indoc! {r#"
            #[starknet::interface]
            trait IHello<T> {
                fn get(self: @T) -> u128;
                fn increase(ref self: T);
            }

            #[starknet::contract]
            mod Hello {
                use starknet::storage::{StoragePointerReadAccess, StoragePointerWriteAccess};
                use starknet::get_contract_address;
                use super::IHello;

                #[storage]
                struct Storage {
                    counter: u128
                }

                #[constructor]
                fn constructor(ref self: ContractState, value_: u128) {
                    self.counter.write(value_);
                }

                #[abi(embed_v0)]
                impl IncImpl of IHello<ContractState> {
                    fn get(self: @ContractState) -> u128 {
                        self.counter.read()
                    }

                    #[some]
                    fn increase(ref self: ContractState)  {
                        self.counter.write( self.counter.read() + 1 );
                    }
                }
            }

            #[cfg(test)]
            mod tests {
                use array::ArrayTrait;
                use core::result::ResultTrait;
                use core::traits::Into;
                use option::OptionTrait;
                use starknet::syscalls::deploy_syscall;
                use traits::TryInto;

                use super::{IHello, Hello, IHelloDispatcher, IHelloDispatcherTrait};

                #[test]
                fn test_flow() {
                    let calldata = array![100];
                    let (address0, _) = deploy_syscall(
                        Hello::TEST_CLASS_HASH.try_into().unwrap(), 0, calldata.span(), false
                    ).unwrap();

                    let mut contract0 = IHelloDispatcher { contract_address: address0 };

                    assert_eq!(@contract0.get(), @100, "contract0.get() == 100");
                    @contract0.increase();
                    assert_eq!(@contract0.get(), @102, "contract0.get() == 102");
                }
            }

        "#})
        .build(&project);

    Scarb::quick_snapbox()
        .arg("cairo-test")
        // Disable output from Cargo.
        .env("CARGO_TERM_QUIET", "true")
        .current_dir(&project)
        .assert()
        .success()
        .stdout_matches(indoc! {r#"
            [..] Compiling some v1.0.0 ([..]Scarb.toml)
            [..] Compiling test(hello_unittest) hello v1.0.0 ([..]Scarb.toml)
            [..]Finished `dev` profile target(s) in [..]
            testing hello ...
            running 1 test
            test hello::tests::test_flow ... ok (gas usage est.: [..])
            test result: ok. 1 passed; 0 failed; 0 ignored; 0 filtered out;

        "#});
}
