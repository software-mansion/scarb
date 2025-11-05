use assert_fs::TempDir;
use assert_fs::fixture::PathChild;
use indoc::indoc;
use scarb_test_support::cairo_plugin_project_builder::CairoPluginProjectBuilder;
use scarb_test_support::command::Scarb;
use scarb_test_support::project_builder::ProjectBuilder;
use scarb_test_support::workspace_builder::WorkspaceBuilder;
use snapbox::Assert;

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
    Assert::new().eq(last, r#"[..] Finished `dev` profile target(s) in [..]"#);
    let (last, _lines) = lines.split_last().unwrap();
    // Line from Cargo output
    Assert::new().eq(
        last,
        r#"[..]Finished `release` profile [optimized] target(s) in[..]"#,
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
    Assert::new().eq(
        last,
        r#"[..] Finished checking `dev` profile target(s) in [..]"#,
    );
    let (last, _lines) = lines.split_last().unwrap();
    // Line from Cargo output
    Assert::new().eq(
        last,
        r#"[..]Finished `release` profile [optimized] target(s) in[..]"#,
    );
}

#[test]
fn can_check_cairo_project_with_plugins() {
    let temp = TempDir::new().unwrap();
    let t = temp.child("some");
    CairoPluginProjectBuilder::default().build(&t);
    let project = temp.child("hello");
    let y = project.child("other");
    CairoPluginProjectBuilder::default().name("other").build(&y);
    WorkspaceBuilder::start()
        .add_member("other")
        .package(
            ProjectBuilder::start()
                .name("hello")
                .version("1.0.0")
                .dep("some", &t),
        )
        .build(&project);
    Scarb::quick_snapbox()
        .arg("check")
        // Disable output from Cargo.
        .env("CARGO_TERM_QUIET", "true")
        .current_dir(&project)
        .assert()
        .success()
        .stdout_eq(indoc! {r#"
            [..]Checking other v1.0.0 ([..]Scarb.toml)
            [..]Compiling some v1.0.0 ([..]Scarb.toml)
            [..]Checking hello v1.0.0 ([..]Scarb.toml)
            [..]Finished checking `dev` profile target(s) in [..]
        "#});
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
    Assert::new().eq(
        first,
        r#"{"status":"checking","message":"some v1.0.0 ([..]Scarb.toml)"}"#,
    );
    let (last, lines) = lines.split_last().unwrap();
    Assert::new().eq(
        last,
        r#"{"status":"finished","message":"checking `dev` profile target(s) in [..]"}"#,
    );
    // Line from Cargo.
    let (last, _lines) = lines.split_last().unwrap();
    Assert::new().eq(last, r#"{"reason":"build-finished","success":true}"#);
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
        .stdout_eq(indoc! {r#"
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
        .stdout_eq(indoc! {r#"
        error: failed to parse manifest at: [..]/Scarb.toml

        Caused by:
            target `cairo-plugin` cannot be mixed with other targets
        "#});
}

#[test]
fn can_define_multiple_macros() {
    let temp = TempDir::new().unwrap();
    let t = temp.child("some");
    CairoPluginProjectBuilder::default()
        .lib_rs(indoc! {r##"
        use cairo_lang_macro::{ProcMacroResult, TokenStream, attribute_macro, AuxData, PostProcessContext, post_process, TokenTree, Token, TextSpan};

        #[attribute_macro]
        pub fn hello(_attr: TokenStream, token_stream: TokenStream) -> ProcMacroResult {
            let new_token_string = token_stream.to_string().replace("12", "34");
            let token_stream = TokenStream::new(vec![TokenTree::Ident(Token::new(
                new_token_string.clone(),
                TextSpan { start: 0, end: new_token_string.len() as u32 },
            ))]);
            let aux_data = AuxData::new(Vec::new());
            ProcMacroResult::new(token_stream).with_aux_data(aux_data)
        }

        #[attribute_macro]
        pub fn world(_attr: TokenStream, token_stream: TokenStream) -> ProcMacroResult {
            let new_token_string = token_stream.to_string().replace("56", "78");
            let token_stream = TokenStream::new(vec![TokenTree::Ident(Token::new(
                new_token_string.clone(),
                TextSpan { start: 0, end: new_token_string.len() as u32 },
            ))]);
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
        use cairo_lang_macro::{ProcMacroResult, TokenStream, attribute_macro, AuxData, PostProcessContext, post_process, TokenTree, Token, TextSpan};

        #[attribute_macro]
        pub fn beautiful(_attr: TokenStream, token_stream: TokenStream) -> ProcMacroResult {
            let new_token_string = token_stream.to_string().replace("90", "09");
            let token_stream = TokenStream::new(vec![TokenTree::Ident(Token::new(
                new_token_string.clone(),
                TextSpan { start: 0, end: new_token_string.len() as u32 },
            ))]);
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
        .dep_cairo_execute()
        .manifest_extra(indoc! {r#"
            [executable]

            [cairo]
            enable-gas = false
        "#})
        .lib_cairo(indoc! {r#"
            #[hello]
            #[beautiful]
            #[world]
            #[executable]
            fn main() -> felt252 { 12 + 56 + 90 }
        "#})
        .build(&project);

    Scarb::quick_snapbox()
        .arg("execute")
        // Disable output from Cargo.
        .env("CARGO_TERM_QUIET", "true")
        .current_dir(&project)
        .assert()
        .success()
        .stdout_eq(indoc! {r#"
            [..]Compiling other v1.0.0 ([..]Scarb.toml)
            [..]Compiling some v1.0.0 ([..]Scarb.toml)
            [..]Compiling hello v1.0.0 ([..]Scarb.toml)
            [..]Finished `dev` profile target(s) in [..]
            [..]Executing hello
            Saving output to: target/execute/hello/execution1
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

    let p = temp.child("pkg");
    CairoPluginProjectBuilder::default()
        .name("pkg")
        .lib_rs(indoc! {r#"
        use cairo_lang_macro::{ProcMacroResult, TokenStream, attribute_macro};

        #[attribute_macro]
        pub fn foo(_attr: TokenStream, token_stream: TokenStream) -> ProcMacroResult {
            ProcMacroResult::new(token_stream)
        }
        "#})
        .build(&p);

    let project = temp.child("hello");
    ProjectBuilder::start()
        .name("hello")
        .version("1.0.0")
        .dep_starknet()
        .dep("some", &t)
        .dep("other", &w)
        .dep("pkg", &p)
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
        .stdout_eq(indoc! {r#"
            [..]Compiling other v1.0.0 ([..]Scarb.toml)
            [..]Compiling pkg v1.0.0 ([..]Scarb.toml)
            [..]Compiling some v1.0.0 ([..]Scarb.toml)
            [..]Compiling hello v1.0.0 ([..]Scarb.toml)
            error: duplicate expansions defined for procedural macros: hello (other v1.0.0 ([..]Scarb.toml) and some v1.0.0 ([..]Scarb.toml))
        "#});
}
#[test]
fn cannot_use_undefined_macro() {
    let temp = TempDir::new().unwrap();
    let t = temp.child("some");
    CairoPluginProjectBuilder::default().build(&t);
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
        .stdout_eq(indoc! {r#"
        [..]Compiling some v1.0.0 ([..]Scarb.toml)
        [..]Compiling hello v1.0.0 ([..]Scarb.toml)
        error: Plugin diagnostic: Unsupported attribute.
         --> [..]lib.cairo:1:1
        #[world]
        ^^^^^^^^

        error: could not compile `hello` due to [..] previous error
        "#});
}

#[test]
fn can_disallow_loading_macros() {
    let temp = TempDir::new().unwrap();
    let t = temp.child("some");
    CairoPluginProjectBuilder::default()
        .lib_rs(indoc! {r##"
        use cairo_lang_macro::{ProcMacroResult, TokenStream, attribute_macro, TokenTree, Token, TextSpan};

        #[attribute_macro]
        pub fn hello(_attr: TokenStream, token_stream: TokenStream) -> ProcMacroResult {
            let new_token_string = token_stream.to_string().replace("12", "34");
            let token_stream = TokenStream::new(vec![TokenTree::Ident(Token::new(
                new_token_string.clone(),
                TextSpan { start: 0, end: new_token_string.len() as u32 },
            ))]);
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
        .arg("--no-proc-macros")
        .arg("build")
        // Disable output from Cargo.
        .env("CARGO_TERM_QUIET", "true")
        .current_dir(&project)
        .assert()
        .failure()
        .stdout_eq(indoc! {r#"
            [..]Compiling hello v1.0.0 ([..]Scarb.toml)
            error: procedural macros are disallowed with `--no-proc-macros` flag
        "#});
}

#[test]
fn only_compiles_needed_macros() {
    let t = TempDir::new().unwrap();
    let some = t.child("some");
    CairoPluginProjectBuilder::default()
        .name("some")
        .build(&some);
    let other = t.child("other");
    CairoPluginProjectBuilder::default()
        .name("other")
        .build(&other);
    let hello = t.child("hello");
    ProjectBuilder::start()
        .name("hello")
        .version("1.0.0")
        .dep("some", &some)
        .build(&hello);
    WorkspaceBuilder::start()
        .add_member("other")
        .add_member("some")
        .add_member("hello")
        .build(&t);
    Scarb::quick_snapbox()
        .arg("build")
        .args(vec!["-p", "hello"])
        // Disable output from Cargo.
        .env("CARGO_TERM_QUIET", "true")
        .current_dir(&t)
        .assert()
        .success()
        .stdout_eq(indoc! {r#"
            [..]Compiling some v1.0.0 ([..]Scarb.toml)
            [..]Compiling hello v1.0.0 ([..]Scarb.toml)
            [..]Finished `dev` profile target(s) in [..]
        "#});
}

#[test]
fn always_compile_macros_requested_with_package_filter() {
    let t = TempDir::new().unwrap();
    let some = t.child("some");
    CairoPluginProjectBuilder::default()
        .name("some")
        .build(&some);
    let other = t.child("other");
    CairoPluginProjectBuilder::default()
        .name("other")
        .build(&other);
    let hello = t.child("hello");
    ProjectBuilder::start()
        .name("hello")
        .version("1.0.0")
        .dep("some", &some)
        .build(&hello);
    WorkspaceBuilder::start()
        .add_member("other")
        .add_member("some")
        .add_member("hello")
        .build(&t);
    Scarb::quick_snapbox()
        .arg("build")
        .arg("--workspace")
        // Disable output from Cargo.
        .env("CARGO_TERM_QUIET", "true")
        .current_dir(&t)
        .assert()
        .success()
        .stdout_eq(indoc! {r#"
            [..]Compiling other v1.0.0 ([..]Scarb.toml)
            [..]Compiling some v1.0.0 ([..]Scarb.toml)
            [..]Compiling hello v1.0.0 ([..]Scarb.toml)
            [..]Finished `dev` profile target(s) in [..]
        "#});
}
