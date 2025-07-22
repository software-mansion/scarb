use assert_fs::TempDir;
use assert_fs::fixture::ChildPath;
use assert_fs::prelude::{FileWriteStr, PathChild};
use indoc::indoc;
use scarb_test_support::cairo_plugin_project_builder::CairoPluginProjectBuilder;
use scarb_test_support::command::{Scarb, ScarbSnapboxExt};
use scarb_test_support::fsx::ChildPathEx;
use scarb_test_support::project_builder::ProjectBuilder;

const DIGEST_LENGTH: usize = 13;

#[test]
fn incremental_artifacts_emitted() {
    // We affix cache dir location, as the corelib path is part of the fingerprint.
    let cache_dir = TempDir::new().unwrap().child("c");

    let t = TempDir::new().unwrap();
    ProjectBuilder::start().name("hello").build(&t);
    ProjectBuilder::start()
        .name("inner")
        .build(&t.child("src/inner"));

    Scarb::quick_snapbox()
        .scarb_cache(cache_dir.path())
        .arg("build")
        .current_dir(&t)
        .assert()
        .success();

    assert_eq!(t.child("target").files(), vec!["CACHEDIR.TAG", "dev"]);
    assert_eq!(
        t.child("target/dev").files(),
        vec![".fingerprint", "hello.sierra.json", "incremental",]
    );

    // We search the dir, as fingerprints will change with different temp dir, so we cannot hardcode
    // the name here.
    let component_id = component_id_factory(t.child("target/dev/"));
    let core_component_id = component_id("core");
    let hello_component_id = component_id("hello");

    assert_eq!(
        t.child("target/dev/incremental").files(),
        vec![
            format!("{core_component_id}.bin"),
            format!("{hello_component_id}.bin")
        ]
    );
    assert_eq!(
        t.child("target/dev/.fingerprint").files(),
        vec![core_component_id.as_str(), hello_component_id.as_str()]
    );
    assert_eq!(
        t.child(format!("target/dev/.fingerprint/{core_component_id}"))
            .files(),
        vec!["core"]
    );
    assert_eq!(
        t.child(format!("target/dev/.fingerprint/{hello_component_id}"))
            .files(),
        vec!["hello"]
    );
    let digest = digest_factory(t.child("target/dev"));
    let core_component_digest = digest(&core_component_id);
    let hello_component_digest = digest(&hello_component_id);

    // Modify the inner package.
    t.child("src/inner/src/lib.cairo")
        .write_str("fn f() -> felt252 { 412 }")
        .unwrap();

    Scarb::quick_snapbox()
        .scarb_cache(cache_dir.path())
        .arg("build")
        .current_dir(&t)
        .assert()
        .success();

    // The project has not been modified, so the incremental artifacts should not change.
    assert_eq!(t.child("target").files(), vec!["CACHEDIR.TAG", "dev"]);
    assert_eq!(
        t.child("target/dev").files(),
        vec![".fingerprint", "hello.sierra.json", "incremental",]
    );
    assert_eq!(
        t.child("target/dev/incremental").files(),
        vec![
            format!("{core_component_id}.bin"),
            format!("{hello_component_id}.bin")
        ]
    );
    assert_eq!(
        t.child("target/dev/.fingerprint").files(),
        vec![core_component_id.as_str(), hello_component_id.as_str()]
    );
    assert_eq!(
        t.child(format!("target/dev/.fingerprint/{core_component_id}"))
            .files(),
        vec!["core"]
    );
    assert_eq!(
        t.child(format!("target/dev/.fingerprint/{hello_component_id}"))
            .files(),
        vec!["hello"]
    );
    assert_eq!(digest(&core_component_id), core_component_digest);
    assert_eq!(digest(&hello_component_id), hello_component_digest);
}

#[test]
fn deps_are_fingerprinted() {
    let cache_dir = TempDir::new().unwrap().child("c");
    let t = TempDir::new().unwrap();

    let first = t.child("first");
    let third = t.child("third");
    let fifth = t.child("fifth");
    ProjectBuilder::start()
        .name("first")
        .dep("second", t.child("second"))
        .dep("fourth", t.child("fourth"))
        .build(&first);

    ProjectBuilder::start()
        .name("second")
        .dep("third", &third)
        .build(&t.child("second"));
    ProjectBuilder::start().name("third").build(&third);

    ProjectBuilder::start()
        .name("fourth")
        .dep("fifth", &fifth)
        .build(&t.child("fourth"));
    ProjectBuilder::start().name("fifth").build(&fifth);

    Scarb::quick_snapbox()
        .scarb_cache(cache_dir.path())
        .arg("build")
        .current_dir(&first)
        .assert()
        .success();

    let fingerprints = || t.child("first/target/dev/.fingerprint").files();
    assert_eq!(fingerprints().len(), 6); // core, first, second, third, fourth, fifth
    let component_id = component_id_factory(first.child("target/dev"));
    let digest = digest_factory(first.child("target/dev"));

    let first_component_id = component_id("first");
    let first_digest = digest(first_component_id.as_str());

    let third_component_id = component_id("third");
    let third_digest = digest(third_component_id.as_str());

    let fourth_component_id = component_id("fourth");
    let fourth_digest = digest(fourth_component_id.as_str());

    // Modify the third package.
    ProjectBuilder::start()
        .name("third")
        .edition("2023_01")
        .build(&third);
    // Modify the fifth package.
    fifth
        .child("src/lib.cairo")
        .write_str("fn f() -> felt252 { 42 }")
        .unwrap();

    Scarb::quick_snapbox()
        .scarb_cache(cache_dir.path())
        .arg("build")
        .current_dir(&first)
        .assert()
        .success();

    assert_ne!(digest(first_component_id.as_str()), first_digest);
    assert_ne!(digest(fourth_component_id.as_str()), fourth_digest);
    assert_eq!(digest(third_component_id.as_str()), third_digest);

    // Note we have changed the edition of the third.
    // Since editions are part of the fingerprint id, the third will change its id.
    assert_eq!(fingerprints().len(), 7); // core, first, second, third, third, fourth, fifth
    let new_third_component_id = fingerprints()
        .iter()
        .find(|t| t.starts_with("third-") && t.as_str() != third_component_id)
        .unwrap()
        .to_string();
    assert_ne!(digest(new_third_component_id.as_str()), third_digest);
}

#[test]
fn can_fingerprint_dependency_cycles() {
    let cache_dir = TempDir::new().unwrap().child("c");
    let target_dir = TempDir::new().unwrap().child("t");
    let t = TempDir::new().unwrap();

    let first = t.child("first");
    let third = t.child("third");
    ProjectBuilder::start()
        .name("first")
        .dep("second", t.child("second"))
        .build(&first);
    ProjectBuilder::start()
        .name("second")
        .dep("third", &third)
        .build(&t.child("second"));
    ProjectBuilder::start()
        .name("third")
        .dep("fourth", t.child("fourth"))
        .build(&third);
    ProjectBuilder::start()
        .name("fourth")
        .dep("first", &first)
        .build(&t.child("fourth"));

    Scarb::quick_snapbox()
        .scarb_cache(cache_dir.path())
        .env("SCARB_TARGET_DIR", target_dir.path())
        .arg("build")
        .current_dir(&first)
        .assert()
        .success();

    let fingerprints = || target_dir.child("dev/.fingerprint").files();
    assert_eq!(fingerprints().len(), 5); // core, first, second, third, fourth

    let component_id = component_id_factory(target_dir.child("dev"));
    let digest = digest_factory(target_dir.child("dev"));

    let first_component_id = component_id("first");
    let first_digest = digest(first_component_id.as_str());

    let second_component_id = component_id("second");
    let second_digest = digest(second_component_id.as_str());

    let third_component_id = component_id("third");
    let third_digest = digest(third_component_id.as_str());

    let fourth_component_id = component_id("fourth");
    let fourth_digest = digest(fourth_component_id.as_str());

    // Modify the third package.
    third
        .child("src/lib.cairo")
        .write_str("fn f() -> felt252 { 412 }")
        .unwrap();

    Scarb::quick_snapbox()
        .scarb_cache(cache_dir.path())
        .env("SCARB_TARGET_DIR", target_dir.path())
        .arg("build")
        .current_dir(&third)
        .assert()
        .success();

    assert_eq!(fingerprints().len(), 5);
    assert_ne!(digest(third_component_id.as_str()), third_digest);
    assert_ne!(digest(first_component_id.as_str()), first_digest);
    assert_ne!(digest(second_component_id.as_str()), second_digest);
    assert_ne!(digest(fourth_component_id.as_str()), fourth_digest);
}

#[test]
fn proc_macros_are_fingerprinted() {
    let cache_dir = TempDir::new().unwrap().child("c");
    let temp = TempDir::new().unwrap();
    let t = temp.child("some");
    CairoPluginProjectBuilder::default()
        .lib_rs(indoc! {r##"
        use cairo_lang_macro::{ProcMacroResult, TokenStream, attribute_macro, TokenTree, Token, TextSpan};

        #[attribute_macro]
        pub fn some(_attr: TokenStream, token_stream: TokenStream) -> ProcMacroResult {
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
        .dep("some", &t)
        .lib_cairo(indoc! {r#"
            #[some]
            fn main() -> felt252 { 12 }
        "#})
        .build(&project);

    Scarb::quick_snapbox()
        .scarb_cache(&cache_dir)
        .arg("build")
        // Disable output from Cargo.
        .env("CARGO_TERM_QUIET", "true")
        .current_dir(&project)
        .assert()
        .success()
        .stdout_matches(indoc! {r#"
            [..] Compiling some v1.0.0 ([..]Scarb.toml)
            [..] Compiling hello v1.0.0 ([..]Scarb.toml)
            [..]Finished `dev` profile target(s) in [..]
        "#});

    let fingerprints = || project.child("target/dev/.fingerprint").files();
    // Note we do not emit cache artifacts for macros, so we don't need their fingerprint files either.
    assert_eq!(fingerprints().len(), 2); // core, hello

    let component_id = component_id_factory(project.child("target/dev/"));
    let digest = digest_factory(project.child("target/dev/"));

    let hello_component_id = component_id("hello");
    let hello_digest = digest(hello_component_id.as_str());

    // Rebuild without changing the macro.
    Scarb::quick_snapbox()
        .scarb_cache(&cache_dir)
        .arg("build")
        // Disable output from Cargo.
        .env("CARGO_TERM_QUIET", "true")
        .current_dir(&project)
        .assert()
        .success()
        .stdout_matches(indoc! {r#"
            [..] Compiling some v1.0.0 ([..]Scarb.toml)
            [..] Compiling hello v1.0.0 ([..]Scarb.toml)
            [..]Finished `dev` profile target(s) in [..]
        "#});
    assert_eq!(fingerprints().len(), 2);
    assert_eq!(digest(hello_component_id.as_str()), hello_digest);

    // Modify the macro.
    t.child("src/lib.rs")
        .write_str(indoc! {r#"
        use cairo_lang_macro::{ProcMacroResult, TokenStream, attribute_macro, TokenTree, Token, TextSpan};

        #[attribute_macro]
        pub fn some(_attr: TokenStream, token_stream: TokenStream) -> ProcMacroResult {
            let new_token_string = token_stream.to_string().replace("12", "56");
            // Changed here:                                              ^^^^
            let token_stream = TokenStream::new(vec![TokenTree::Ident(Token::new(
                new_token_string.clone(),
                TextSpan { start: 0, end: new_token_string.len() as u32 },
            ))]);
            ProcMacroResult::new(token_stream)
        }
        "#})
        .unwrap();

    Scarb::quick_snapbox()
        .scarb_cache(&cache_dir)
        .arg("build")
        // Disable output from Cargo.
        .env("CARGO_TERM_QUIET", "true")
        .current_dir(&project)
        .assert()
        .success()
        .stdout_matches(indoc! {r#"
            [..] Compiling some v1.0.0 ([..]Scarb.toml)
            [..] Compiling hello v1.0.0 ([..]Scarb.toml)
            [..]Finished `dev` profile target(s) in [..]
        "#});
    assert_eq!(fingerprints().len(), 2);
    assert_ne!(digest(hello_component_id.as_str()), hello_digest);
}

fn component_id_factory(target_dir: ChildPath) -> impl Fn(&str) -> String {
    move |name: &str| {
        let component_id = target_dir
            .child(".fingerprint")
            .files()
            .iter()
            .find(|t| t.starts_with(&format!("{name}-")))
            .unwrap_or_else(|| panic!("failed to find component id for {name}"))
            .to_string();
        assert_eq!(component_id.len(), name.len() + DIGEST_LENGTH + 1); // 1 for the dash
        component_id
    }
}

fn digest_factory(target_dir: ChildPath) -> impl Fn(&str) -> String {
    move |component_id: &str| {
        let (name, _) = component_id.split_once("-").unwrap();
        let digest = target_dir
            .child(format!(".fingerprint/{component_id}/{name}"))
            .read_to_string();
        assert_eq!(digest.len(), DIGEST_LENGTH);
        digest
    }
}
