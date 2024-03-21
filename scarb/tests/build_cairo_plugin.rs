use assert_fs::fixture::{FileWriteStr, PathChild};
use assert_fs::TempDir;
use camino::Utf8PathBuf;
use indoc::{formatdoc, indoc};
use snapbox::assert_matches;
use std::collections::HashMap;
use std::path::PathBuf;

use scarb_test_support::command::Scarb;
use scarb_test_support::fsx;
use scarb_test_support::project_builder::ProjectBuilder;

struct CairoPluginProjectBuilder {
    project: ProjectBuilder,
    src: HashMap<Utf8PathBuf, String>,
}

impl CairoPluginProjectBuilder {
    pub fn start() -> Self {
        Self {
            project: ProjectBuilder::start(),
            src: Default::default(),
        }
    }

    pub fn scarb_project(mut self, mutate: impl FnOnce(ProjectBuilder) -> ProjectBuilder) -> Self {
        self.project = mutate(self.project);
        self
    }

    pub fn src(mut self, path: impl Into<Utf8PathBuf>, source: impl ToString) -> Self {
        self.src.insert(path.into(), source.to_string());
        self
    }

    pub fn lib_rs(self, source: impl ToString) -> Self {
        self.src("src/lib.rs", source.to_string())
    }

    pub fn just_code(&self, t: &impl PathChild) {
        for (path, source) in &self.src {
            t.child(path).write_str(source).unwrap();
        }
    }

    pub fn build(&self, t: &impl PathChild) {
        self.project.just_manifest(t);
        self.just_code(t);
    }
}

fn lib_path(lib_name: &str) -> String {
    let path = fsx::canonicalize(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../plugins/")
            .join(lib_name),
    )
    .unwrap();
    serde_json::to_string(&path).unwrap()
}

fn simple_project_with_code(t: &impl PathChild, code: impl ToString) {
    let macro_lib_path = lib_path("cairo-lang-macro");
    let macro_stable_lib_path = lib_path("cairo-lang-macro-stable");
    CairoPluginProjectBuilder::start()
        .scarb_project(|b| {
            b.name("some")
                .version("1.0.0")
                .manifest_extra(r#"[cairo-plugin]"#)
        })
        .lib_rs(code)
        .src(
            "Cargo.toml",
            formatdoc! {r#"
        [package]
        name = "some"
        version = "0.1.0"
        edition = "2021"
        publish = false

        [lib]
        crate-type = ["rlib","cdylib"]

        [dependencies]
        cairo-lang-macro = {{ path = {macro_lib_path}}}
        cairo-lang-macro-stable = {{ path = {macro_stable_lib_path}}}
        serde = {{ version = "*", features = ["derive"] }}
        serde_json = "*"
        "#},
        )
        .build(t);
}

fn simple_project(t: &impl PathChild) {
    let code = indoc! {r#"
        use cairo_lang_macro::{ProcMacroResult, TokenStream, attribute_macro, AuxData};

        #[attribute_macro]
        pub fn some(token_stream: TokenStream) -> ProcMacroResult {
            let _code = token_stream.to_string();
            ProcMacroResult::Leave { diagnostics: Vec::new() }
        }
        "#};
    simple_project_with_code(t, code);
}

#[test]
fn compile_cairo_plugin() {
    let t = TempDir::new().unwrap();
    simple_project(&t);
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
    assert_matches(r#"[..] Finished release target(s) in [..]"#, last);
    let (last, _lines) = lines.split_last().unwrap();
    // Line from Cargo output
    assert_matches(r#"[..]Finished release [optimized] target(s) in[..]"#, last);
}

#[test]
fn check_cairo_plugin() {
    let t = TempDir::new().unwrap();
    simple_project(&t);
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
    assert_matches(r#"[..] Finished checking release target(s) in [..]"#, last);
    let (last, _lines) = lines.split_last().unwrap();
    // Line from Cargo output
    assert_matches(r#"[..]Finished release [optimized] target(s) in[..]"#, last);
}

#[test]
fn resolve_fetched_plugins() {
    let t = TempDir::new().unwrap();
    simple_project(&t);
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
    simple_project(&t);
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
        r#"{"status":"finished","message":"checking release target(s) in [..]"}"#,
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
    simple_project_with_code(
        &t,
        indoc! {r#"
        use cairo_lang_macro::{ProcMacroResult, TokenStream, attribute_macro, Diagnostic};
        
        #[attribute_macro]
        pub fn some(token_stream: TokenStream) -> ProcMacroResult {
            let _code = token_stream.to_string();
            let diag = Diagnostic::warn("Some warning from macro.");
            ProcMacroResult::Leave { diagnostics: vec![diag] }
        }
        "#},
    );
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

            [..]Finished release target(s) in [..]
        "#});
}

#[test]
fn can_emit_plugin_error() {
    let temp = TempDir::new().unwrap();
    let t = temp.child("some");
    simple_project_with_code(
        &t,
        indoc! {r#"
        use cairo_lang_macro::{ProcMacroResult, TokenStream, attribute_macro, Diagnostic};

        #[attribute_macro]
        pub fn some(token_stream: TokenStream) -> ProcMacroResult {
            let _code = token_stream.to_string();
            let diag = Diagnostic::error("Some error from macro.");
            ProcMacroResult::Leave { diagnostics: vec![diag] }
        }
        "#},
    );
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
fn can_remove_original_node() {
    let temp = TempDir::new().unwrap();
    let t = temp.child("some");
    simple_project_with_code(
        &t,
        indoc! {r#"
        use cairo_lang_macro::{ProcMacroResult, TokenStream, attribute_macro};

        #[attribute_macro]
        pub fn some(_: TokenStream) -> ProcMacroResult {
            ProcMacroResult::Remove { diagnostics: Vec::new() }
        }
        "#},
    );
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
            [..]Finished release target(s) in [..]
            [..]Running hello
            Run completed successfully, returning [34]
        "#});
}

#[test]
fn can_replace_original_node() {
    let temp = TempDir::new().unwrap();
    let t = temp.child("some");
    simple_project_with_code(
        &t,
        indoc! {r##"
        use cairo_lang_macro::{ProcMacroResult, TokenStream, attribute_macro};

        #[attribute_macro]
        pub fn some(token_stream: TokenStream) -> ProcMacroResult {
            let token_stream = TokenStream::new(
                token_stream
                    .to_string()
                    // Remove macro call to avoid infinite loop.
                    .replace("#[some]", "")
                    .replace("12", "34")
            );
            ProcMacroResult::Replace {
                token_stream,
                aux_data: None,
                diagnostics: Vec::new()
            }
        }
        "##},
    );
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
            [..]Finished release target(s) in [..]
            [..]Running hello
            Run completed successfully, returning [34]
        "#});
}

#[test]
fn can_return_aux_data_from_plugin() {
    let temp = TempDir::new().unwrap();
    let t = temp.child("some");
    simple_project_with_code(
        &t,
        indoc! {r##"
        use cairo_lang_macro::{ProcMacroResult, TokenStream, attribute_macro, AuxData, post_process};
        use serde::{Serialize, Deserialize};

        #[derive(Debug, Serialize, Deserialize)]
        struct SomeMacroDataFormat {
            msg: String
        }

        #[attribute_macro]
        pub fn some(token_stream: TokenStream) -> ProcMacroResult {
            let token_stream = TokenStream::new(
                token_stream
                    .to_string()
                    // Remove macro call to avoid infinite loop.
                    .replace("#[some]", "")
                    .replace("12", "34")
            );
            let value = SomeMacroDataFormat { msg: "Hello from some macro!".to_string() };
            let value = serde_json::to_string(&value).unwrap();
            let value: Vec<u8> = value.into_bytes();
            let aux_data = AuxData::new(value);

            ProcMacroResult::replace(token_stream, Some(aux_data))
        }

        #[post_process]
        pub fn callback(aux_data: Vec<AuxData>) {
            let aux_data = aux_data.into_iter()
                .map(|aux_data| {
                    let value: Vec<u8> = aux_data.into();
                    let aux_data: SomeMacroDataFormat = serde_json::from_slice(&value).unwrap();
                    aux_data
                })
                .collect::<Vec<_>>();
            println!("{:?}", aux_data);
        }
        "##},
    );

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
            [..]Finished release target(s) in [..]
        "#});
}

#[test]
fn can_read_token_stream_metadata() {
    let temp = TempDir::new().unwrap();
    let t = temp.child("some");
    simple_project_with_code(
        &t,
        indoc! {r##"
        use cairo_lang_macro::{ProcMacroResult, TokenStream, attribute_macro};

        #[attribute_macro]
        pub fn some(token_stream: TokenStream) -> ProcMacroResult {
            println!("{:#?}", token_stream.metadata());
            ProcMacroResult::Leave { diagnostics: Vec::new() }
        }

        "##},
    );

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
            [..]Finished release target(s) in [..]
        "#});
}
