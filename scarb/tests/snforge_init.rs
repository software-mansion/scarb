use assert_fs::fixture::PathChild;
use assert_fs::TempDir;
use scarb::core::TomlManifest;
use scarb_test_support::command::Scarb;
use scarb_test_support::fsx::AssertFsUtf8Ext;

#[test]
#[ignore = "run this test by name"]
fn new_simple() {
    let pt = TempDir::new().unwrap();

    Scarb::quick_snapbox()
        .arg("new")
        .arg("hello")
        .args(["--test-runner", "starknet-foundry"])
        .current_dir(&pt)
        .assert()
        .success();

    let t = pt.child("hello");
    assert!(t.is_dir());
    assert!(t.child("Scarb.toml").is_file());
    assert!(t.child("src/lib.cairo").is_file());
    assert!(t.child(".gitignore").is_file());
    assert!(t.child("tests").is_dir());
    assert!(t.child("tests/test_contract.cairo").is_file());
    assert!(t.child(".git").is_dir());

    let toml_manifest = TomlManifest::read_from_path(t.child("Scarb.toml").utf8_path()).unwrap();
    assert_eq!(toml_manifest.package.unwrap().name.as_str(), "hello");
    let deps = toml_manifest.dependencies.unwrap();
    assert_eq!(deps.len(), 1);
    assert!(deps.contains_key("starknet"));
    let deps = toml_manifest.dev_dependencies.unwrap();
    assert_eq!(deps.len(), 2);
    assert!(deps.contains_key("snforge_std"));
    assert!(deps.contains_key("assert_macros"));
    assert_eq!(
        toml_manifest
            .scripts
            .unwrap()
            .get("test")
            .unwrap()
            .as_defined()
            .unwrap()
            .to_string(),
        "snforge test"
    );

    Scarb::quick_snapbox()
        .arg("check")
        .current_dir(&t)
        .assert()
        .success();
}
