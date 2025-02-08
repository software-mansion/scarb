use assert_fs::fixture::PathChild;
use assert_fs::TempDir;
use indoc::indoc;
use scarb_test_support::cairo_plugin_project_builder::CairoPluginProjectBuilder;
use scarb_test_support::command::Scarb;
use scarb_test_support::project_builder::ProjectBuilder;
use scarb_test_support::workspace_builder::WorkspaceBuilder;
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
        .stdout_matches(indoc! {r#"
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
            warn: `scarb cairo-run` will be deprecated soon
            help: use `scarb execute` instead
            [..]Compiling other v1.0.0 ([..]Scarb.toml)
            [..]Compiling some v1.0.0 ([..]Scarb.toml)
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
            [..]Compiling other v1.0.0 ([..]Scarb.toml)
            [..]Compiling some v1.0.0 ([..]Scarb.toml)
            [..]Compiling hello v1.0.0 ([..]Scarb.toml)
            error: duplicate expansions defined for procedural macros: hello (some v1.0.0 ([..]Scarb.toml) and other v1.0.0 ([..]Scarb.toml))
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
        .stdout_matches(indoc! {r#"
        [..]Compiling some v1.0.0 ([..]Scarb.toml)
        [..]Compiling hello v1.0.0 ([..]Scarb.toml)
        error: Plugin diagnostic: Unsupported attribute.
         --> [..]lib.cairo:1:1
        #[world]
        ^^^^^^^^

        error: could not compile `hello` due to previous error
        "#});
}
