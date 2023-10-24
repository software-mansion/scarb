use assert_fs::prelude::*;
use assert_fs::TempDir;
use indoc::indoc;

use scarb_metadata::Metadata;
use scarb_test_support::command::{CommandExt, Scarb};
use scarb_test_support::fsx::ChildPathEx;
use scarb_test_support::project_builder::ProjectBuilder;

#[test]
fn build_defaults_to_dev() {
    let t = TempDir::new().unwrap();
    ProjectBuilder::start().name("hello").build(&t);

    Scarb::quick_snapbox()
        .arg("build")
        .current_dir(&t)
        .assert()
        .success();

    assert_eq!(t.child("target").files(), vec!["CACHEDIR.TAG", "dev"]);
    assert_eq!(t.child("target/dev").files(), vec!["hello.sierra.json"]);
}

#[test]
fn can_build_release() {
    let t = TempDir::new().unwrap();
    ProjectBuilder::start().name("hello").build(&t);

    Scarb::quick_snapbox()
        .args(["--release", "build"])
        .current_dir(&t)
        .assert()
        .success();

    assert_eq!(t.child("target").files(), vec!["CACHEDIR.TAG", "release"]);
    assert_eq!(t.child("target/release").files(), vec!["hello.sierra.json"]);
}

#[test]
fn defaults_to_dev() {
    let t = TempDir::new().unwrap();
    ProjectBuilder::start().name("hello").build(&t);

    let metadata = Scarb::quick_snapbox()
        .args(["--json", "metadata", "--format-version", "1"])
        .current_dir(&t)
        .stdout_json::<Metadata>();

    let mut all_profiles = metadata.profiles;
    all_profiles.sort();

    assert_eq!(metadata.current_profile, "dev".to_string());
    assert_eq!(all_profiles, vec!["dev".to_string(), "release".to_string()]);
}

#[test]
fn can_choose_release() {
    let t = TempDir::new().unwrap();
    ProjectBuilder::start().name("hello").build(&t);

    let metadata = Scarb::quick_snapbox()
        .args(["--json", "--release", "metadata", "--format-version", "1"])
        .current_dir(&t)
        .stdout_json::<Metadata>();

    let mut all_profiles = metadata.profiles;
    all_profiles.sort();

    assert_eq!(metadata.current_profile, "release".to_string());
    assert_eq!(all_profiles, vec!["dev".to_string(), "release".to_string()]);
}

#[test]
fn can_choose_dev() {
    let t = TempDir::new().unwrap();
    ProjectBuilder::start().name("hello").build(&t);

    let metadata = Scarb::quick_snapbox()
        .args(["--json", "--dev", "metadata", "--format-version", "1"])
        .current_dir(&t)
        .stdout_json::<Metadata>();

    let mut all_profiles = metadata.profiles;
    all_profiles.sort();

    assert_eq!(metadata.current_profile, "dev".to_string());
    assert_eq!(all_profiles, vec!["dev".to_string(), "release".to_string()]);
}

#[test]
fn cannot_choose_both_dev_and_release() {
    let t = TempDir::new().unwrap();
    ProjectBuilder::start().name("hello").build(&t);

    Scarb::quick_snapbox()
        .args(["--dev", "--release","metadata", "--format-version", "1"])
        .current_dir(&t)
        .assert()
        .failure()
        .stderr_matches(indoc! {r#"
            error: the argument '--dev' cannot be used with '--release'

            Usage: scarb[..] --dev --global-cache-dir <DIRECTORY> --global-config-dir <DIRECTORY> <COMMAND>

            For more information, try '--help'.
        "#});
}

#[test]
fn can_choose_release_by_name() {
    let t = TempDir::new().unwrap();
    ProjectBuilder::start().name("hello").build(&t);

    let metadata = Scarb::quick_snapbox()
        .args([
            "--json",
            "--profile",
            "release",
            "metadata",
            "--format-version",
            "1",
        ])
        .current_dir(&t)
        .stdout_json::<Metadata>();

    let mut all_profiles = metadata.profiles;
    all_profiles.sort();

    assert_eq!(metadata.current_profile, "release".to_string());
    assert_eq!(all_profiles, vec!["dev".to_string(), "release".to_string()]);
}

#[test]
fn cannot_choose_both_release_and_by_name() {
    let t = TempDir::new().unwrap();
    ProjectBuilder::start().name("hello").build(&t);

    Scarb::quick_snapbox()
        .args(["--release", "--profile", "dev","metadata", "--format-version", "1"])
        .current_dir(&t)
        .assert()
        .failure()
        .stderr_matches(indoc! {r#"
            error: the argument '--release' cannot be used with '--profile <PROFILE>'

            Usage: scarb[..] --release --global-cache-dir <DIRECTORY> --global-config-dir <DIRECTORY> <COMMAND>

            For more information, try '--help'.
        "#});
}

#[test]
fn can_choose_dev_by_name() {
    let t = TempDir::new().unwrap();
    ProjectBuilder::start().name("hello").build(&t);

    let metadata = Scarb::quick_snapbox()
        .args([
            "--json",
            "--profile",
            "dev",
            "metadata",
            "--format-version",
            "1",
        ])
        .current_dir(&t)
        .stdout_json::<Metadata>();

    let mut all_profiles = metadata.profiles;
    all_profiles.sort();

    assert_eq!(metadata.current_profile, "dev".to_string());
    assert_eq!(all_profiles, vec!["dev".to_string(), "release".to_string()]);
}

#[test]
fn can_choose_dev_by_short_name() {
    let t = TempDir::new().unwrap();
    ProjectBuilder::start().name("hello").build(&t);

    let metadata = Scarb::quick_snapbox()
        .args(["--json", "-P", "dev", "metadata", "--format-version", "1"])
        .current_dir(&t)
        .stdout_json::<Metadata>();

    let mut all_profiles = metadata.profiles;
    all_profiles.sort();

    assert_eq!(metadata.current_profile, "dev".to_string());
    assert_eq!(all_profiles, vec!["dev".to_string(), "release".to_string()]);
}

#[test]
fn can_choose_custom_profile() {
    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello")
        .manifest_extra(indoc! {r#"
            [profile.custom]
        "#})
        .build(&t);

    let metadata = Scarb::quick_snapbox()
        .args([
            "--json",
            "--profile",
            "custom",
            "metadata",
            "--format-version",
            "1",
        ])
        .current_dir(&t)
        .stdout_json::<Metadata>();

    let mut all_profiles = metadata.profiles;
    all_profiles.sort();

    assert_eq!(metadata.current_profile, "custom".to_string());
    assert_eq!(
        all_profiles,
        vec![
            "custom".to_string(),
            "dev".to_string(),
            "release".to_string()
        ]
    );
}

#[test]
fn cannot_choose_not_existing_profile() {
    let t = TempDir::new().unwrap();
    ProjectBuilder::start().name("hello").build(&t);

    Scarb::quick_snapbox()
        .args(["--profile", "custom", "metadata", "--format-version", "1"])
        .current_dir(&t)
        .assert()
        .failure()
        .stdout_matches("error: workspace `[..]` has no profile `custom`\n");
}

#[test]
fn can_use_short_profile_args_in_scripts() {
    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello")
        .manifest_extra(
            r#"
            [scripts]
            build-dev = "scarb --dev build"
            build-release = "scarb --release build"
            "#,
        )
        .build(&t);

    Scarb::quick_snapbox()
        .args(["run", "build-dev"])
        .current_dir(&t)
        .assert()
        .success();

    Scarb::quick_snapbox()
        .args(["run", "build-release"])
        .current_dir(&t)
        .assert()
        .success();
}

#[test]
fn sierra_replace_ids_defaults_true_in_dev() {
    let t = TempDir::new().unwrap();
    ProjectBuilder::start().name("hello").build(&t);

    let metadata = Scarb::quick_snapbox()
        .args(["--json", "metadata", "--format-version", "1"])
        .current_dir(&t)
        .stdout_json::<Metadata>();

    assert_eq!(metadata.current_profile, "dev".to_string());
    assert!(!metadata.compilation_units.is_empty());
    for cu in metadata.compilation_units {
        let compiler_config = cu.compiler_config;
        assert!(compiler_config
            .get("sierra_replace_ids")
            .unwrap()
            .as_bool()
            .unwrap());
    }
}

#[test]
fn sierra_replace_ids_default_false_in_release() {
    let t = TempDir::new().unwrap();
    ProjectBuilder::start().name("hello").build(&t);

    let metadata = Scarb::quick_snapbox()
        .args([
            "--json",
            "--profile",
            "release",
            "metadata",
            "--format-version",
            "1",
        ])
        .current_dir(&t)
        .stdout_json::<Metadata>();

    assert_eq!(metadata.current_profile, "release".to_string());
    assert!(!metadata.compilation_units.is_empty());
    for cu in metadata.compilation_units {
        let compiler_config = cu.compiler_config;
        assert!(!compiler_config
            .get("sierra_replace_ids")
            .unwrap()
            .as_bool()
            .unwrap());
    }
}

#[test]
fn compiler_config_set_for_all_profiles() {
    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello")
        .manifest_extra(
            r#"
            [cairo]
            sierra-replace-ids = true

            [profile.some-profile]
            "#,
        )
        .build(&t);

    let metadata = Scarb::quick_snapbox()
        .args(["--json", "metadata", "--format-version", "1"])
        .current_dir(&t)
        .stdout_json::<Metadata>();

    assert_eq!(metadata.current_profile, "dev".to_string());
    assert!(!metadata.compilation_units.is_empty());
    for cu in metadata.compilation_units {
        let compiler_config = cu.compiler_config;
        assert!(compiler_config
            .get("sierra_replace_ids")
            .unwrap()
            .as_bool()
            .unwrap());
    }

    let metadata = Scarb::quick_snapbox()
        .args(["--json", "--release", "metadata", "--format-version", "1"])
        .current_dir(&t)
        .stdout_json::<Metadata>();

    assert_eq!(metadata.current_profile, "release".to_string());
    assert!(!metadata.compilation_units.is_empty());
    for cu in metadata.compilation_units {
        let compiler_config = cu.compiler_config;
        assert!(compiler_config
            .get("sierra_replace_ids")
            .unwrap()
            .as_bool()
            .unwrap());
    }

    let metadata = Scarb::quick_snapbox()
        .args([
            "--json",
            "--profile",
            "some-profile",
            "metadata",
            "--format-version",
            "1",
        ])
        .current_dir(&t)
        .stdout_json::<Metadata>();

    assert_eq!(metadata.current_profile, "some-profile".to_string());
    assert!(!metadata.compilation_units.is_empty());
    for cu in metadata.compilation_units {
        let compiler_config = cu.compiler_config;
        assert!(compiler_config
            .get("sierra_replace_ids")
            .unwrap()
            .as_bool()
            .unwrap());
    }
}

#[test]
fn can_set_replace_ids_in_profile() {
    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello")
        .manifest_extra(indoc! {r#"
            [profile.release.cairo]
            sierra-replace-ids = true
        "#})
        .build(&t);

    let metadata = Scarb::quick_snapbox()
        .args(["--json", "--release", "metadata", "--format-version", "1"])
        .current_dir(&t)
        .stdout_json::<Metadata>();

    assert_eq!(metadata.current_profile, "release".to_string());
    assert!(!metadata.compilation_units.is_empty());
    for cu in metadata.compilation_units {
        let compiler_config = cu.compiler_config;
        assert!(compiler_config
            .get("sierra_replace_ids")
            .unwrap()
            .as_bool()
            .unwrap());
    }
}

#[test]
fn profile_precedes_compiler_config() {
    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello")
        .manifest_extra(indoc! {r#"
            [cairo]
            sierra-replace-ids = false

            [profile.release.cairo]
            sierra-replace-ids = true
        "#})
        .build(&t);

    let metadata = Scarb::quick_snapbox()
        .args(["--json", "--release", "metadata", "--format-version", "1"])
        .current_dir(&t)
        .stdout_json::<Metadata>();

    assert_eq!(metadata.current_profile, "release".to_string());
    assert!(!metadata.compilation_units.is_empty());
    for cu in metadata.compilation_units {
        let compiler_config = cu.compiler_config;
        assert!(compiler_config
            .get("sierra_replace_ids")
            .unwrap()
            .as_bool()
            .unwrap());
    }

    let metadata = Scarb::quick_snapbox()
        .args(["--json", "metadata", "--format-version", "1"])
        .current_dir(&t)
        .stdout_json::<Metadata>();

    assert_eq!(metadata.current_profile, "dev".to_string());
    assert!(!metadata.compilation_units.is_empty());
    for cu in metadata.compilation_units {
        let compiler_config = cu.compiler_config;
        assert!(!compiler_config
            .get("sierra_replace_ids")
            .unwrap()
            .as_bool()
            .unwrap());
    }
}

#[test]
fn custom_profiles_inherit_from_dev_by_default() {
    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello")
        .manifest_extra(indoc! {r#"
            [profile.custom]
        "#})
        .build(&t);

    let metadata = Scarb::quick_snapbox()
        .args([
            "--json",
            "--profile",
            "custom",
            "metadata",
            "--format-version",
            "1",
        ])
        .current_dir(&t)
        .stdout_json::<Metadata>();

    assert_eq!(metadata.current_profile, "custom".to_string());
    assert!(!metadata.compilation_units.is_empty());
    for cu in metadata.compilation_units {
        let compiler_config = cu.compiler_config;
        assert!(compiler_config
            .get("sierra_replace_ids")
            .unwrap()
            .as_bool()
            .unwrap());
    }
}

#[test]
fn custom_profiles_can_inherit_by_name() {
    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello")
        .manifest_extra(indoc! {r#"
            [profile.custom]
            inherits = "release"
        "#})
        .build(&t);

    let metadata = Scarb::quick_snapbox()
        .args([
            "--json",
            "--profile",
            "custom",
            "metadata",
            "--format-version",
            "1",
        ])
        .current_dir(&t)
        .stdout_json::<Metadata>();

    assert_eq!(metadata.current_profile, "custom".to_string());
    assert!(!metadata.compilation_units.is_empty());
    for cu in metadata.compilation_units {
        let compiler_config = cu.compiler_config;
        assert!(!compiler_config
            .get("sierra_replace_ids")
            .unwrap()
            .as_bool()
            .unwrap());
    }
}

#[test]
fn custom_profiles_can_inherit_dev_and_release_only() {
    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello")
        .manifest_extra(indoc! {r#"
            [profile.some-profile]

            [profile.custom]
            inherits = "some-profile"
        "#})
        .build(&t);

    Scarb::quick_snapbox()
        .args(["--profile", "custom", "metadata", "--format-version", "1"])
        .current_dir(&t)
        .assert()
        .failure()
        .stdout_matches(indoc! {r#"
            error: failed to parse manifest at: [..]

            Caused by:
                profile can inherit from `dev` or `release` only, found `some-profile`
        "#});
}

#[test]
fn profile_overrides_tool() {
    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello")
        .manifest_extra(indoc! {r#"
            [tool.snforge]
            some-key = "some-value"

            [profile.release.tool.snforge]
            some-key = "some-other-value"
        "#})
        .build(&t);
    let metadata = Scarb::quick_snapbox()
        .args(["--json", "metadata", "--format-version", "1"])
        .current_dir(&t)
        .stdout_json::<Metadata>();

    assert_eq!(metadata.current_profile, "dev".to_string());
    assert_eq!(metadata.packages.len(), 3);

    let package = metadata.packages[1].clone();
    assert_eq!(
        package
            .manifest_metadata
            .tool
            .unwrap()
            .get("snforge")
            .unwrap()
            .get("some-key")
            .unwrap()
            .as_str()
            .unwrap(),
        "some-value"
    );

    let metadata = Scarb::quick_snapbox()
        .args(["--json", "--release", "metadata", "--format-version", "1"])
        .current_dir(&t)
        .stdout_json::<Metadata>();

    assert_eq!(metadata.current_profile, "release".to_string());
    assert_eq!(metadata.packages.len(), 3);

    let package = metadata.packages[1].clone();
    assert_eq!(
        package
            .manifest_metadata
            .tool
            .unwrap()
            .get("snforge")
            .unwrap()
            .get("some-key")
            .unwrap()
            .as_str()
            .unwrap(),
        "some-other-value"
    );
}
