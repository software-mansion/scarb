use assert_fs::TempDir;
use assert_fs::assert::PathAssert;
use assert_fs::fixture::PathChild;
use indoc::indoc;
use itertools::Itertools;
use scarb_metadata::{Cfg, DepKind, Metadata};
use scarb_test_support::command::{CommandExt, Scarb};
use scarb_test_support::gitx;
use scarb_test_support::project_builder::{Dep, DepBuilder, ProjectBuilder};
use scarb_test_support::registry::local::LocalRegistry;
use scarb_test_support::workspace_builder::WorkspaceBuilder;

fn build_example_program(t: &TempDir) {
    ProjectBuilder::start()
        .name("hello")
        .manifest_extra(indoc! {r#"
            [features]
            x = []
            y = []
            "#})
        .lib_cairo(indoc! {r#"
            #[cfg(feature: 'x')]
            fn f() -> felt252 { 21 }

            #[cfg(feature: 'y')]
            fn f() -> felt252 { 59 }

            fn main() -> felt252 {
                f()
            }
        "#})
        .build(t);
}

fn build_missing_manifest_example_program(t: &TempDir) {
    ProjectBuilder::start()
        .name("hello")
        .lib_cairo(indoc! {r#"
            #[cfg(feature: 'x')]
            fn f() -> felt252 { 21 }

            #[cfg(feature: 'y')]
            fn f() -> felt252 { 59 }

            fn main() -> felt252 {
                f()
            }
        "#})
        .build(t);
}

fn build_incorrect_manifest_feature_example_program(t: &TempDir) {
    ProjectBuilder::start()
        .name("hello")
        .manifest_extra(indoc! {r#"
            [features]
            8x = []
            y = []
            "#})
        .lib_cairo(indoc! {r#"
            #[cfg(feature: '8x')]
            fn f() -> felt252 { 21 }

            #[cfg(feature: 'y')]
            fn f() -> felt252 { 59 }

            fn main() -> felt252 {
                f()
            }
        "#})
        .build(t);
}

fn build_with_default_features(t: &TempDir) {
    ProjectBuilder::start()
        .name("hello")
        .manifest_extra(indoc! {r#"
            [features]
            default = ["x", "y"]
            x = []
            y = []
            "#})
        .lib_cairo(indoc! {r#"
            #[cfg(feature: 'x')]
            fn g() -> felt252 { 21 }

            #[cfg(feature: 'y')]
            fn f() -> felt252 { g() }

            fn main() -> felt252 {
                f()
            }
        "#})
        .build(t);
}

fn build_with_all_features_required(t: &TempDir) {
    ProjectBuilder::start()
        .name("hello")
        .manifest_extra(indoc! {r#"
            [features]
            w = []
            x = []
            y = []
            z = []
            "#})
        .lib_cairo(indoc! {r#"
            #[cfg(feature: 'x')]
            fn f() -> felt252 { 22 }

            #[cfg(feature: 'y')]
            fn g() -> felt252 { f() }

            #[cfg(feature: 'z')]
            fn h() -> felt252 { g() }

            #[cfg(feature: 'w')]
            fn i() -> felt252 { h() }

            fn main() -> felt252 {
                i()
            }
        "#})
        .build(t);
}

#[test]
fn features_success() {
    let t = TempDir::new().unwrap();
    build_example_program(&t);
    Scarb::quick_snapbox()
        .arg("build")
        .arg("--features")
        .arg("x")
        .current_dir(&t)
        .assert()
        .success();

    t.child("target/dev/hello.sierra.json")
        .assert(predicates::str::contains(r#""debug_name":"hello::f""#));

    build_example_program(&t);
    Scarb::quick_snapbox()
        .arg("build")
        .arg("--features")
        .arg("y")
        .current_dir(&t)
        .assert()
        .success();

    t.child("target/dev/hello.sierra.json")
        .assert(predicates::str::contains(r#""debug_name":"hello::f""#));
}

#[test]
fn features_fail_both_features_enabled() {
    let t = TempDir::new().unwrap();
    build_example_program(&t);
    Scarb::quick_snapbox()
        .arg("build")
        .arg("--features")
        .arg("x,y")
        .current_dir(&t)
        .assert()
        .stdout_matches(indoc! {r#"
            [..] Compiling hello v1.0.0 ([..])
            error: The name `f` is defined multiple times.
             --> [..]/src/lib.cairo[..]
            fn f() -> felt252 { 59 }
               ^
            
            error: could not compile `hello` due to previous error
        "#})
        .failure();
}

#[test]
fn features_fail_no_feature_enabled() {
    let t = TempDir::new().unwrap();
    build_example_program(&t);
    Scarb::quick_snapbox()
        .arg("build")
        .current_dir(&t)
        .assert()
        .stdout_matches(indoc! {r#"
            [..] Compiling hello v1.0.0 ([..])
            error[E0006]: Function not found.
             --> [..]/src/lib.cairo[..]
                f()
                ^

            error: could not compile `hello` due to previous error
        "#})
        .failure();
}

#[test]
fn features_unknown_feature() {
    let t = TempDir::new().unwrap();
    build_example_program(&t);
    Scarb::quick_snapbox()
        .arg("build")
        .arg("--features")
        .arg("z")
        .current_dir(&t)
        .assert()
        .stdout_matches(indoc! {r#"
            error: none of the selected packages contains `z` feature
            note: to use features, you need to define [features] section in Scarb.toml
        "#})
        .failure();
}

#[test]
fn features_fail_missing_manifest() {
    let t = TempDir::new().unwrap();
    build_missing_manifest_example_program(&t);
    Scarb::quick_snapbox()
        .arg("build")
        .arg("--features")
        .arg("x")
        .current_dir(&t)
        .assert()
        .stdout_matches(indoc! {r#"
            error: none of the selected packages contains `x` feature
            note: to use features, you need to define [features] section in Scarb.toml
        "#})
        .failure();
}

#[test]
fn features_fail_incorrect_manifest() {
    let t = TempDir::new().unwrap();
    build_incorrect_manifest_feature_example_program(&t);
    Scarb::quick_snapbox()
        .arg("build")
        .arg("--features")
        .arg("x")
        .current_dir(&t)
        .assert()
        .stdout_matches(indoc! {r#"
            error: failed to parse manifest at: [..]/Scarb.toml

            Caused by:
                TOML parse error at line 9, column 1
                  |
                9 | 8x = []
                  | ^^
                the name `8x` cannot be used as a package name, names cannot start with a digit
        "#})
        .failure();
}

#[test]
fn features_with_default_features() {
    let t = TempDir::new().unwrap();
    build_with_default_features(&t);
    Scarb::quick_snapbox()
        .arg("build")
        .current_dir(&t)
        .assert()
        .success();

    t.child("target/dev/hello.sierra.json")
        .assert(predicates::str::contains(r#""debug_name":"hello::main""#));
}

#[test]
fn features_no_default_features() {
    let t = TempDir::new().unwrap();
    build_with_default_features(&t);
    Scarb::quick_snapbox()
        .arg("build")
        .arg("--no-default-features")
        .current_dir(&t)
        .assert()
        .stdout_matches(indoc! {r#"
            [..] Compiling hello v1.0.0 ([..])
            error[E0006]: Function not found.
             --> [..]/src/lib.cairo[..]
                f()
                ^

            error: could not compile `hello` due to previous error
        "#})
        .failure();
}

#[test]
fn features_all_features() {
    let t = TempDir::new().unwrap();
    build_with_all_features_required(&t);
    Scarb::quick_snapbox()
        .arg("build")
        .arg("--all-features")
        .current_dir(&t)
        .assert()
        .success();

    t.child("target/dev/hello.sierra.json")
        .assert(predicates::str::contains(r#""debug_name":"hello::main""#));
}

#[test]
fn features_all_features_failing() {
    let t = TempDir::new().unwrap();
    build_with_all_features_required(&t);
    Scarb::quick_snapbox()
        .arg("build")
        .current_dir(&t)
        .assert()
        .stdout_matches(indoc! {r#"
            [..] Compiling hello v1.0.0 ([..])
            error[E0006]: Function not found.
             --> [..]/src/lib.cairo[..]
                i()
                ^

            error: could not compile `hello` due to previous error
        "#})
        .failure();
}

#[test]
fn features_no_default_and_all_failing() {
    let t = TempDir::new().unwrap();
    build_with_default_features(&t);
    Scarb::quick_snapbox()
        .arg("build")
        .arg("--no-default-features")
        .arg("--all-features")
        .current_dir(&t)
        .assert()
        .stderr_matches(indoc! {r#"
            error: the argument '--no-default-features' cannot be used with '--all-features'

            Usage: scarb[..] build --no-default-features

            For more information, try '--help'.
        "#})
        .failure();
}

#[test]
fn features_metadata_feature_in_compilation_units() {
    let t = TempDir::new().unwrap();
    build_example_program(&t);
    let output = Scarb::quick_snapbox()
        .arg("--json")
        .arg("metadata")
        .arg("--features")
        .arg("x")
        .arg("--format-version")
        .arg("1")
        .current_dir(&t)
        .stdout_json::<Metadata>();

    assert!(!output.compilation_units.is_empty());
    let unit = &output.compilation_units[0];
    assert!(unit.package.repr.starts_with("hello "));
    assert_eq!(unit.target.name, "hello");
    assert!(!unit.components.is_empty());
    assert!(
        unit.cfg
            .contains(&Cfg::KV("target".into(), unit.target.kind.clone()))
    );
    assert!(unit.components.len() >= 2);
    let main_component_cfg = unit.components[1].cfg.clone();
    assert!(
        main_component_cfg.is_some_and(|cfg| cfg.contains(&Cfg::KV("feature".into(), "x".into())))
    );
}

#[test]
fn features_in_workspace_success() {
    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("first")
        .manifest_extra(indoc! {r#"
            [features]
            x = []
            y = []
            "#})
        .lib_cairo(indoc! {r#"
            #[cfg(feature: 'x')]
            fn f() -> felt252 { 21 }

            #[cfg(feature: 'y')]
            fn f() -> felt252 { 59 }

            fn main() -> felt252 {
                f()
            }
        "#})
        .build(&t.child("first"));
    ProjectBuilder::start()
        .name("second")
        .lib_cairo(indoc! {r#"
            fn main() -> felt252 {
                12
            }
        "#})
        .build(&t.child("second"));
    WorkspaceBuilder::start()
        .add_member("first")
        .add_member("second")
        .build(&t);

    Scarb::quick_snapbox()
        .arg("check")
        .arg("--package")
        .arg("first")
        .arg("--features")
        .arg("x")
        .current_dir(&t)
        .assert()
        .success();
}

#[test]
fn features_in_workspace_validated() {
    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("first")
        .manifest_extra(indoc! {r#"
            [features]
            x = []
            y = []
            "#})
        .lib_cairo(indoc! {r#"
            #[cfg(feature: 'x')]
            fn f() -> felt252 { 21 }

            #[cfg(feature: 'y')]
            fn f() -> felt252 { 59 }

            fn main() -> felt252 {
                f()
            }
        "#})
        .build(&t.child("first"));
    ProjectBuilder::start()
        .name("second")
        .lib_cairo(indoc! {r#"
            fn main() -> felt252 {
                12
            }
        "#})
        .build(&t.child("second"));
    WorkspaceBuilder::start()
        .add_member("first")
        .add_member("second")
        .build(&t);

    Scarb::quick_snapbox()
        .arg("check")
        .arg("--package")
        .arg("second")
        .arg("--features")
        .arg("x")
        .current_dir(&t)
        .assert()
        .failure()
        .stdout_matches(indoc! {r#"
            error: none of the selected packages contains `x` feature
            note: to use features, you need to define [features] section in Scarb.toml
        "#});
}

#[test]
fn parse_dependency_features_simple() {
    let t = TempDir::new().unwrap();

    let path_dep = t.child("path_dep");
    ProjectBuilder::start()
        .name("path_dep")
        .version("0.1.0")
        .manifest_extra(indoc! {r#"
                [features]
                first = []
                second = []
            "#})
        .build(&path_dep);

    let git_dep = gitx::new("git_dep", |t| {
        ProjectBuilder::start()
            .name("git_dep")
            .version("0.2.0")
            .manifest_extra(indoc! {r#"
                [features]
                first = []
                second = []
            "#})
            .build(&t);
    });

    let mut registry = LocalRegistry::create();
    registry.publish(|t| {
        ProjectBuilder::start()
            .name("registry_dep")
            .version("1.0.0")
            .manifest_extra(indoc! {r#"
                [features]
                first = []
                second = []
            "#})
            .build(t);
    });

    ProjectBuilder::start()
        .name("hello")
        .version("1.0.0")
        .dep(
            "registry_dep",
            Dep.version("1.0.0")
                .registry(&registry)
                .features(vec!["second", "first"].into_iter())
                .default_features(false),
        )
        .dep(
            "path_dep",
            path_dep
                .version("0.1.0")
                .default_features(true)
                .features(vec!["second", "first"].into_iter()),
        )
        .dep(
            "git_dep",
            git_dep
                .version("0.2.0")
                .features(vec!["second", "first"].into_iter()),
        )
        .build(&t);

    let meta = Scarb::quick_snapbox()
        .arg("--json")
        .arg("metadata")
        .arg("--format-version=1")
        .current_dir(&t)
        .stdout_json::<Metadata>();

    let hello = meta
        .packages
        .into_iter()
        .find(|p| p.name == "hello")
        .unwrap();

    let registry_dep = hello
        .dependencies
        .iter()
        .find(|d| d.name == "registry_dep")
        .unwrap();

    assert_eq!(
        registry_dep.features,
        Some(vec!["first".to_string(), "second".to_string()])
    );
    assert_eq!(registry_dep.default_features, Some(false));

    let path_dep = hello
        .dependencies
        .iter()
        .find(|d| d.name == "path_dep")
        .unwrap();

    assert_eq!(
        path_dep.features,
        Some(vec!["first".to_string(), "second".to_string()])
    );
    assert_eq!(path_dep.default_features, Some(true));

    let git_dep = hello
        .dependencies
        .iter()
        .find(|d| d.name == "git_dep")
        .unwrap();

    assert_eq!(
        git_dep.features,
        Some(vec!["first".to_string(), "second".to_string()])
    );
    // Note there is no `default-features` field in the dependency specification, this is the default.
    assert_eq!(git_dep.default_features, Some(true));
}

#[test]
fn parse_dependency_features_invalid() {
    let t = TempDir::new().unwrap();

    let mut registry = LocalRegistry::create();
    registry.publish(|t| {
        ProjectBuilder::start()
            .name("registry_dep")
            .version("1.0.0")
            .build(t);
    });

    ProjectBuilder::start()
        .name("hello")
        .version("1.0.0")
        .dep(
            "registry_dep",
            Dep.version("1.0.0")
                .registry(&registry)
                .features(vec!["8x"].into_iter())
                .default_features(false),
        )
        .build(&t);

    Scarb::quick_snapbox()
        .arg("metadata")
        .arg("--format-version=1")
        .current_dir(&t)
        .assert()
        .failure()
        .stdout_matches(indoc! {r#"
            error: failed to parse manifest at: [..]Scarb.toml

            Caused by:
                the name `8x` cannot be used as a package name, names cannot start with a digit
        "#});
}

#[test]
fn can_declare_same_dependency_with_different_kinds_and_features() {
    let t = TempDir::new().unwrap();

    let mut registry = LocalRegistry::create();
    registry.publish(|t| {
        ProjectBuilder::start()
            .name("registry_dep")
            .version("1.0.0")
            .manifest_extra(indoc! {r#"
                [features]
                first = []
                second = []
            "#})
            .build(t);
    });

    ProjectBuilder::start()
        .name("hello")
        .version("1.0.0")
        .dep(
            "registry_dep",
            Dep.version("1.0.0")
                .registry(&registry)
                .features(vec!["first"].into_iter()),
        )
        .dev_dep(
            "registry_dep",
            Dep.version("1.0.0")
                .registry(&registry)
                .features(vec!["second"].into_iter()),
        )
        .build(&t);

    let meta = Scarb::quick_snapbox()
        .arg("--json")
        .arg("metadata")
        .arg("--format-version=1")
        .current_dir(&t)
        .stdout_json::<Metadata>();

    let package = meta.packages.iter().find(|p| p.name == "hello").unwrap();

    let dep = package
        .dependencies
        .iter()
        .filter(|d| d.name == "registry_dep")
        .collect_vec();

    assert_eq!(dep.len(), 2);

    let normal = dep.iter().find(|d| d.kind.is_none()).unwrap();
    assert_eq!(normal.features, Some(vec!["first".to_string()]));

    let dev = dep.iter().find(|d| d.kind == Some(DepKind::Dev)).unwrap();
    assert_eq!(dev.features, Some(vec!["second".to_string()]));

    let lib_cu = meta
        .compilation_units
        .iter()
        .find(|cu| cu.target.kind == "lib")
        .unwrap();
    let lib_component = lib_cu
        .components
        .iter()
        .find(|component| component.name == "registry_dep")
        .unwrap();
    assert_eq!(
        lib_component.cfg.as_ref().unwrap(),
        &vec![
            Cfg::KV("feature".to_string(), "first".to_string()),
            Cfg::KV("target".to_string(), "lib".to_string())
        ],
    );

    let test_cu = meta
        .compilation_units
        .iter()
        .find(|cu| cu.target.kind == "test")
        .unwrap();
    let test_component = test_cu
        .components
        .iter()
        .find(|component| component.name == "registry_dep")
        .unwrap();
    assert_eq!(
        test_component.cfg.as_ref().unwrap(),
        &vec![
            Cfg::KV("feature".to_string(), "first".to_string()),
            Cfg::KV("feature".to_string(), "second".to_string()),
            Cfg::KV("target".to_string(), "test".to_string()),
        ],
    );
}

#[test]
fn cannot_use_not_existing_features_in_deps() {
    let t = TempDir::new().unwrap();

    let path_dep = t.child("path_dep");
    ProjectBuilder::start()
        .name("path_dep")
        .version("0.1.0")
        .manifest_extra(indoc! {r#"
                [features]
                first = []
                second = []
            "#})
        .build(&path_dep);

    ProjectBuilder::start()
        .name("hello")
        .version("1.0.0")
        .dep(
            "path_dep",
            path_dep
                .version("0.1.0")
                .default_features(true)
                .features(vec!["first", "third"].into_iter()),
        )
        .build(&t);

    Scarb::quick_snapbox()
        .arg("check")
        .current_dir(&t)
        .assert()
        .failure()
        .stdout_eq("error: unknown features: third\n");
}

#[test]
fn dependency_features_simple() {
    let t = TempDir::new().unwrap();

    let path_dep = t.child("path_dep");
    ProjectBuilder::start()
        .name("path_dep")
        .version("0.1.0")
        .manifest_extra(indoc! {r#"
            [features]
            x = []
            y = []
        "#})
        .lib_cairo(indoc! {r#"
            #[cfg(feature: 'x')]
            fn g() -> felt252 { 21 }

            #[cfg(feature: 'y')]
            fn f() -> felt252 { g() }

            pub fn main() -> felt252 {
                f()
            }
        "#})
        .build(&path_dep);

    let git_dep = gitx::new("git_dep", |t| {
        ProjectBuilder::start()
            .name("git_dep")
            .version("0.2.0")
            .manifest_extra(indoc! {r#"
                [features]
                x = []
                y = []
            "#})
            .lib_cairo(indoc! {r#"
                #[cfg(feature: 'x')]
                fn g() -> felt252 { 21 }
    
                #[cfg(feature: 'y')]
                fn f() -> felt252 { g() }
    
                pub fn main() -> felt252 {
                    f()
                }
            "#})
            .build(&t);
    });

    let mut registry = LocalRegistry::create();
    registry.publish(|t| {
        ProjectBuilder::start()
            .name("registry_dep")
            .version("1.0.0")
            .manifest_extra(indoc! {r#"
                [features]
                x = []
                y = []
            "#})
            .lib_cairo(indoc! {r#"
                #[cfg(feature: 'x')]
                fn g() -> felt252 { 21 }
    
                #[cfg(feature: 'y')]
                fn f() -> felt252 { g() }
    
                pub fn main() -> felt252 {
                    f()
                }
            "#})
            .build(t);
    });

    ProjectBuilder::start()
        .name("hello")
        .version("1.0.0")
        .manifest_extra(indoc! {r#"
            [features]
            x = []
            y = []
        "#})
        .lib_cairo(indoc! {r#"
            #[cfg(feature: 'x')]
            fn g() -> felt252 { path_dep::main() + git_dep::main() + registry_dep::main() }

            #[cfg(feature: 'y')]
            fn f() -> felt252 { g() }

            fn main() -> felt252 {
                f()
            }
        "#})
        .dep(
            "registry_dep",
            Dep.version("1.0.0")
                .registry(&registry)
                .features(vec!["x", "y"].into_iter())
                .default_features(false),
        )
        .dep(
            "path_dep",
            path_dep
                .version("0.1.0")
                .default_features(true)
                .features(vec!["x", "y"].into_iter()),
        )
        .dep(
            "git_dep",
            git_dep
                .version("0.2.0")
                .features(vec!["x", "y"].into_iter())
                .default_features(false),
        )
        .build(&t);

    Scarb::quick_snapbox()
        .arg("check")
        .arg("--features=x,y")
        .current_dir(&t)
        .assert()
        .success()
        .stdout_matches(indoc! {r#"
            [..]Updating git repository [..]git_dep
            [..]Checking hello v1.0.0 ([..]Scarb.toml)
            [..]Finished checking `dev` profile target(s) in[..]
        "#});
}

#[test]
fn dependency_features_in_workspace() {
    let t = TempDir::new().unwrap();
    let pkg1 = t.child("first");
    ProjectBuilder::start()
        .name("first")
        .manifest_extra(indoc! {r#"
            [features]
            x = []
            y = []
        "#})
        .lib_cairo(indoc! {r#"
            #[cfg(feature: 'x')]
            fn g() -> felt252 { 21 }

            #[cfg(feature: 'y')]
            fn f() -> felt252 { g() }

            fn main() -> felt252 {
                f()
            }
        "#})
        .build(&pkg1);
    let pkg2 = t.child("second");
    ProjectBuilder::start()
        .name("second")
        .dep(
            "first",
            Dep.path("../first").features(vec!["x", "y"].into_iter()),
        )
        .build(&pkg2);
    WorkspaceBuilder::start()
        .add_member("first")
        .add_member("second")
        .build(&t);

    Scarb::quick_snapbox()
        .arg("check")
        .arg("--features=x,y")
        .arg("--package=first")
        .current_dir(&t)
        .assert()
        .success()
        .stdout_matches(indoc! {r#"
            [..]Checking first v1.0.0 ([..]Scarb.toml)
            [..]Finished checking `dev` profile target(s) in[..]
        "#});

    Scarb::quick_snapbox()
        .arg("check")
        .arg("--package=second")
        .current_dir(&t)
        .assert()
        .success()
        .stdout_matches(indoc! {r#"
            [..]Checking second v1.0.0 ([..]Scarb.toml)
            [..]Finished checking `dev` profile target(s) in[..]
        "#});
}

#[test]
fn dependency_features_disabled_by_default() {
    let t = TempDir::new().unwrap();

    let path_dep = t.child("path_dep");
    ProjectBuilder::start()
        .name("path_dep")
        .version("0.1.0")
        .manifest_extra(indoc! {r#"
            [features]
            x = []
            y = []
        "#})
        .lib_cairo(indoc! {r#"
            #[cfg(feature: 'x')]
            fn g() -> felt252 { 21 }

            #[cfg(feature: 'y')]
            fn f() -> felt252 { g() }

            pub fn main() -> felt252 {
                f()
            }
        "#})
        .build(&path_dep);

    ProjectBuilder::start()
        .name("hello")
        .version("1.0.0")
        .manifest_extra(indoc! {r#"
            [features]
            x = []
            y = []
        "#})
        .lib_cairo(indoc! {r#"
            #[cfg(feature: 'x')]
            fn g() -> felt252 { path_dep::main() }

            #[cfg(feature: 'y')]
            fn f() -> felt252 { g() }

            fn main() -> felt252 {
                f()
            }
        "#})
        .dep("path_dep", path_dep.version("0.1.0").default_features(true))
        .build(&t);

    Scarb::quick_snapbox()
        .arg("check")
        .arg("--features=x,y")
        .current_dir(&t)
        .assert()
        .failure()
        .stdout_matches(indoc! {r#"
            [..]Checking hello v1.0.0 ([..]Scarb.toml)
            error[E0006]: Function not found.
             --> [..]lib.cairo:8:5
                f()
                ^

            error: could not check `hello` due to previous error
        "#});
}

#[test]
fn dependency_features_can_be_default() {
    let t = TempDir::new().unwrap();

    let path_dep = t.child("path_dep");
    ProjectBuilder::start()
        .name("path_dep")
        .version("0.1.0")
        .manifest_extra(indoc! {r#"
            [features]
            default = ["x", "y"]
            x = []
            y = []
        "#})
        .lib_cairo(indoc! {r#"
            #[cfg(feature: 'x')]
            fn g() -> felt252 { 21 }

            #[cfg(feature: 'y')]
            fn f() -> felt252 { g() }

            pub fn main() -> felt252 {
                f()
            }
        "#})
        .build(&path_dep);

    ProjectBuilder::start()
        .name("hello")
        .version("1.0.0")
        .manifest_extra(indoc! {r#"
            [features]
            x = []
            y = []
        "#})
        .lib_cairo(indoc! {r#"
            #[cfg(feature: 'x')]
            fn g() -> felt252 { path_dep::main() }

            #[cfg(feature: 'y')]
            fn f() -> felt252 { g() }

            fn main() -> felt252 {
                f()
            }
        "#})
        .dep("path_dep", path_dep.version("0.1.0"))
        .build(&t);

    Scarb::quick_snapbox()
        .arg("check")
        .arg("--features=x,y")
        .current_dir(&t)
        .assert()
        .success()
        .stdout_matches(indoc! {r#"
            [..]Checking hello v1.0.0 ([..]Scarb.toml)
            [..]Finished checking `dev` profile target(s) in[..]
        "#});
}

#[test]
fn dependency_features_can_disable_default_features() {
    let t = TempDir::new().unwrap();

    let path_dep = t.child("path_dep");
    ProjectBuilder::start()
        .name("path_dep")
        .version("0.1.0")
        .manifest_extra(indoc! {r#"
            [features]
            default = ["x", "y"]
            x = []
            y = []
        "#})
        .lib_cairo(indoc! {r#"
            #[cfg(feature: 'x')]
            fn g() -> felt252 { 21 }

            #[cfg(feature: 'y')]
            fn f() -> felt252 { g() }

            pub fn main() -> felt252 {
                f()
            }
        "#})
        .build(&path_dep);

    ProjectBuilder::start()
        .name("hello")
        .version("1.0.0")
        .manifest_extra(indoc! {r#"
            [features]
            x = []
            y = []
        "#})
        .lib_cairo(indoc! {r#"
            #[cfg(feature: 'x')]
            fn g() -> felt252 { path_dep::main() }

            #[cfg(feature: 'y')]
            fn f() -> felt252 { g() }

            fn main() -> felt252 {
                f()
            }
        "#})
        .dep(
            "path_dep",
            path_dep.version("0.1.0").default_features(false),
        )
        .build(&t);

    Scarb::quick_snapbox()
        .arg("check")
        .arg("--features=x,y")
        .current_dir(&t)
        .assert()
        .failure()
        .stdout_matches(indoc! {r#"
            [..]Checking hello v1.0.0 ([..]Scarb.toml)
            error[E0006]: Function not found.
             --> [..]lib.cairo:8:5
                f()
                ^

            error: could not check `hello` due to previous error
        "#});
}

#[test]
fn dependency_features_unification() {
    let t = TempDir::new().unwrap();

    // External deps
    let path_dep = t.child("path_dep");
    ProjectBuilder::start()
        .name("path_dep")
        .version("0.1.0")
        .manifest_extra(indoc! {r#"
            [features]
            default = ["x"]
            x = []
            y = []
        "#})
        .lib_cairo(indoc! {r#"
            #[cfg(feature: 'x')]
            fn g() -> felt252 { 21 }

            #[cfg(feature: 'y')]
            fn f() -> felt252 { g() }

            pub fn main() -> felt252 {
                f()
            }
        "#})
        .build(&path_dep);

    // Workspace members
    let ws = TempDir::new().unwrap();
    let first = ws.child("first");
    ProjectBuilder::start()
        .name("first")
        .dep(
            "path_dep",
            Dep.path(path_dep.path().to_string_lossy())
                // Note, that `path_dep` requires two features - `x` and `y`. We enable `y` here
                // explicitly. The `x` is set as default feature in `path_dep`. We do disable default
                // features here, but `second` depends on `path_dep` enabling default features (and
                // nothing else). As a result of unification, we will enable default features
                // (caused by `second`) and `y` (caused by `first`), which means that the package
                // can be compiled successfully.
                .default_features(false)
                .features(["y"].iter()),
        )
        .manifest_extra(indoc! {r#"
            [features]
            x = []
            y = []
        "#})
        .lib_cairo(indoc! {r#"
            #[cfg(feature: 'y')]
            fn g() -> felt252 { 21 }

            #[cfg(feature: 'x')]
            fn f() -> felt252 { g() }

            fn main() -> felt252 {
                f()
            }
        "#})
        .build(&first);

    let second = ws.child("second");
    ProjectBuilder::start()
        .name("second")
        .dep(
            "first",
            Dep.path("../first").features(vec!["x", "y"].into_iter()),
        )
        .dep(
            "path_dep",
            Dep.path(path_dep.path().to_string_lossy())
                .default_features(true),
        )
        .build(&second);

    WorkspaceBuilder::start()
        .add_member("first")
        .add_member("second")
        .package(
            ProjectBuilder::start()
                .name("hello")
                .dep("second", Dep.path("./second"))
                // Note we do not pass the required `x,y` features to `first` here, but the build
                // will still succeed, because the dependency on `first` in `second` does enable them.
                .dep("first", Dep.path("./first")),
        )
        .build(&ws);

    Scarb::quick_snapbox()
        .arg("check")
        .arg("--package=hello")
        .current_dir(&ws)
        .assert()
        .success()
        .stdout_matches(indoc! {r#"
            [..]Checking hello v1.0.0 ([..]Scarb.toml)
            [..]Finished checking `dev` profile target(s) in[..]
        "#});
}

#[test]
fn features_unification_does_not_leak_between_units_for_ws_member() {
    let t = TempDir::new().unwrap();

    // Workspace member
    let ws = TempDir::new().unwrap();
    let first = ws.child("first");
    ProjectBuilder::start()
        .name("first")
        .manifest_extra(indoc! {r#"
            [features]
            x = []
            y = []
        "#})
        .lib_cairo(indoc! {r#"
            #[cfg(feature: 'y')]
            fn g() -> felt252 { 21 }

            #[cfg(feature: 'x')]
            fn f() -> felt252 { g() }

            fn main() -> felt252 {
                f()
            }
        "#})
        .build(&first);

    // External dep
    let path_dep = t.child("path_dep");
    ProjectBuilder::start()
        .name("path_dep")
        .version("0.1.0")
        .dep(
            "first",
            Dep.path(first.path().to_string_lossy())
                .features(["x", "y"].iter()),
        )
        .build(&path_dep);

    // Workspace member
    let second = ws.child("second");
    ProjectBuilder::start()
        .name("second")
        .dep("path_dep", &path_dep)
        .build(&second);

    WorkspaceBuilder::start()
        .add_member("first")
        .add_member("second")
        .build(&ws);

    Scarb::quick_snapbox()
        .arg("check")
        .arg("--package=second")
        .current_dir(&ws)
        .assert()
        .success()
        .stdout_matches(indoc! {r#"
            [..]Checking second v1.0.0 ([..]Scarb.toml)
            [..]Finished checking `dev` profile target(s) in[..]
        "#});

    Scarb::quick_snapbox()
        .arg("check")
        .arg("--package=first")
        .current_dir(&ws)
        .assert()
        .failure()
        .stdout_matches(indoc! {r#"
            [..]Checking first v1.0.0 ([..]Scarb.toml)
            error[E0006]: Function not found.
             --> [..]lib.cairo:8:5
                f()
                ^

            error: could not check `first` due to previous error
        "#});
}

#[test]
fn features_unification_does_not_leak_between_units() {
    let t = TempDir::new().unwrap();

    // External deps
    let path_dep_1 = t.child("path_dep_1");
    ProjectBuilder::start()
        .name("path_dep_1")
        .version("0.1.0")
        .manifest_extra(indoc! {r#"
            [features]
            x = []
            y = []
        "#})
        .lib_cairo(indoc! {r#"
            #[cfg(feature: 'y')]
            fn g() -> felt252 { 21 }

            #[cfg(feature: 'x')]
            fn f() -> felt252 { g() }

            fn main() -> felt252 {
                f()
            }
        "#})
        .build(&path_dep_1);

    let path_dep_2 = t.child("path_dep_2");
    ProjectBuilder::start()
        .name("path_dep_2")
        .version("0.1.0")
        .dep(
            "path_dep_1",
            Dep.path(path_dep_1.path().to_string_lossy())
                .features(["x", "y"].iter()),
        )
        .build(&path_dep_2);

    // Workspace members
    let ws = TempDir::new().unwrap();
    let first = ws.child("first");
    ProjectBuilder::start()
        .name("first")
        .dep("path_dep_1", &path_dep_1)
        .build(&first);
    let second = ws.child("second");
    ProjectBuilder::start()
        .name("second")
        .dep("path_dep_2", &path_dep_2)
        .build(&second);

    WorkspaceBuilder::start()
        .add_member("first")
        .add_member("second")
        .build(&ws);

    Scarb::quick_snapbox()
        .arg("check")
        .arg("--package=first")
        .current_dir(&ws)
        .assert()
        .failure()
        .stdout_matches(indoc! {r#"
            [..]Checking first v1.0.0 ([..]Scarb.toml)
            error[E0006]: Function not found.
             --> [..]lib.cairo:8:5
                f()
                ^
            
            error: could not check `first` due to previous error
        "#});

    Scarb::quick_snapbox()
        .arg("check")
        .arg("--package=second")
        .current_dir(&ws)
        .assert()
        .success()
        .stdout_matches(indoc! {r#"
            [..]Checking second v1.0.0 ([..]Scarb.toml)
            [..]Finished checking `dev` profile target(s) in[..]
        "#});
}

#[test]
fn dependency_features_unification_for_test_target() {
    let t = TempDir::new().unwrap();

    let path_dep = t.child("path_dep");
    ProjectBuilder::start()
        .name("path_dep")
        .version("0.1.0")
        .manifest_extra(indoc! {r#"
            [features]
            default = ["x"]
            x = []
            y = []
        "#})
        .lib_cairo(indoc! {r#"
            #[cfg(feature: 'x')]
            fn g() -> felt252 { 21 }

            #[cfg(feature: 'y')]
            fn f() -> felt252 { g() }

            pub fn main() -> felt252 {
                f()
            }
        "#})
        .build(&path_dep);

    let hello = TempDir::new().unwrap();
    let first = hello.child("first");
    ProjectBuilder::start()
        .name("first")
        .dep(
            "path_dep",
            Dep.path(path_dep.path().to_string_lossy())
                // Note, that `path_dep` requires two features - `x` and `y`. We enable `y` here
                // explicitly. The `x` is set as default feature in `path_dep`. We do disable default
                // features here, but `second` depends on `path_dep` enabling default features (and
                // nothing else). As a result of unification, we will enable default features
                // (caused by `second`) and `y` (caused by `first`), which means that the package
                // can be compiled successfully.
                .default_features(false)
                .features(["y"].iter()),
        )
        .manifest_extra(indoc! {r#"
            [features]
            x = []
            y = []
        "#})
        .lib_cairo(indoc! {r#"
            #[cfg(feature: 'y')]
            fn g() -> felt252 { 21 }

            #[cfg(feature: 'x')]
            fn f() -> felt252 { g() }

            fn main() -> felt252 {
                f()
            }
        "#})
        .build(&first);

    let second = hello.child("second");
    ProjectBuilder::start()
        .name("second")
        .dep(
            "path_dep",
            Dep.path(path_dep.path().to_string_lossy())
                .default_features(true),
        )
        .build(&second);

    ProjectBuilder::start()
        .name("hello")
        .dev_dep("second", Dep.path("./second"))
        .dev_dep(
            "first",
            Dep.path("./first").features(vec!["x", "y"].into_iter()),
        )
        .build(&hello);

    Scarb::quick_snapbox()
        .arg("build")
        .arg("--test")
        .current_dir(&hello)
        .assert()
        .success()
        .stdout_matches(indoc! {r#"
            [..]Compiling test(hello_unittest) hello v1.0.0 ([..]Scarb.toml)
            [..]Finished `dev` profile target(s) in[..]
        "#});
}

#[test]
fn dev_dep_features_do_not_propagate() {
    let t = TempDir::new().unwrap();

    let path_dep = t.child("path_dep");
    ProjectBuilder::start()
        .name("path_dep")
        .version("0.1.0")
        .manifest_extra(indoc! {r#"
            [features]
            x = []
            y = []
        "#})
        .lib_cairo(indoc! {r#"
            #[cfg(feature: 'x')]
            fn g() -> felt252 { 21 }

            #[cfg(feature: 'y')]
            fn f() -> felt252 { g() }

            pub fn main() -> felt252 {
                f()
            }
        "#})
        .build(&path_dep);

    let first = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("first")
        .dev_dep(
            "path_dep",
            Dep.path(path_dep.path().to_string_lossy())
                .features(["x", "y"].iter()),
        )
        .build(&first);

    let hello = TempDir::new().unwrap();

    ProjectBuilder::start()
        .name("hello")
        .dep("first", Dep.path(first.path().to_string_lossy()))
        .dev_dep(
            "path_dep",
            // Note we do not enable any features here.
            // They are enabled in `dev_dep` of `first`, but should not propagate here.
            // If `first` depended on `path_dep` via normal dependency, features would propagate.
            Dep.path(path_dep.path().to_string_lossy()),
        )
        .build(&hello);

    Scarb::quick_snapbox()
        .arg("build")
        .arg("--test")
        .current_dir(&hello)
        .assert()
        .failure()
        .stdout_matches(indoc! {r#"
            [..]Compiling test(hello_unittest) hello v1.0.0 ([..]Scarb.toml)
            error[E0006]: Function not found.
             --> [..]lib.cairo:8:5
                f()
                ^

            error: could not compile `hello` due to previous error
        "#});
}

#[test]
fn can_declare_default_by_name() {
    let t = TempDir::new().unwrap();

    let path_dep = t.child("path_dep");
    ProjectBuilder::start()
        .name("path_dep")
        .version("0.1.0")
        .manifest_extra(indoc! {r#"
            [features]
            default = ["x"]
            x = []
        "#})
        .lib_cairo(indoc! {r#"
            #[cfg(feature: 'x')]
            fn g() -> felt252 { 21 }

            #[cfg(feature: 'default')]
            fn f() -> felt252 { g() }

            pub fn main() -> felt252 {
                f()
            }
        "#})
        .build(&path_dep);

    ProjectBuilder::start()
        .name("hello")
        .version("1.0.0")
        .manifest_extra(indoc! {r#"
            [features]
            default = ["x"]
            x = []
        "#})
        .lib_cairo(indoc! {r#"
            #[cfg(feature: 'x')]
            fn g() -> felt252 { path_dep::main() }

            #[cfg(feature: 'default')]
            fn f() -> felt252 { g() }

            fn main() -> felt252 {
                f()
            }
        "#})
        .dep(
            "path_dep",
            path_dep
                .version("0.1.0")
                .default_features(false)
                .features(vec!["x", "default"].into_iter()),
        )
        .build(&t);

    Scarb::quick_snapbox()
        .arg("check")
        .arg("--features=default")
        .arg("--no-default-features")
        .current_dir(&t)
        .assert()
        .success()
        .stdout_matches(indoc! {r#"
            [..]Checking hello v1.0.0 ([..]Scarb.toml)
            [..]Finished checking `dev` profile target(s) in[..]
        "#});
}

#[test]
fn can_deserialize_features_enabling_dependency_features() {
    let t = TempDir::new().unwrap();

    let first = t.child("path_dep");
    ProjectBuilder::start()
        .name("first")
        .version("0.1.0")
        .manifest_extra(indoc! {r#"
            [features]
            x = []
            y = []
        "#})
        .lib_cairo(indoc! {r#"
            #[cfg(feature: 'x')]
            fn g() -> felt252 { 21 }

            #[cfg(feature: 'y')]
            fn f() -> felt252 { g() }

            pub fn main() -> felt252 {
                f()
            }
        "#})
        .build(&first);

    ProjectBuilder::start()
        .name("hello")
        .version("1.0.0")
        .manifest_extra(indoc! {r#"
            [features]
            z = ["first/x", "first/y"]
        "#})
        .dep("first", &first)
        .build(&t);

    Scarb::quick_snapbox()
        .arg("fetch")
        .current_dir(&t)
        .assert()
        .success();
}
