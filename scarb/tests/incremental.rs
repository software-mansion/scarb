use assert_fs::TempDir;
use assert_fs::prelude::PathChild;
use scarb_test_support::command::Scarb;
use scarb_test_support::fsx::ChildPathEx;
use scarb_test_support::project_builder::ProjectBuilder;

#[test]
fn incremental_artifacts_emitted() {
    let cache_dir = TempDir::new().unwrap().child("c");

    let t = TempDir::new().unwrap();
    ProjectBuilder::start().name("hello").build(&t);

    Scarb::quick_snapbox()
        .env("SCARB_CACHE", cache_dir.path())
        .arg("build")
        .current_dir(&t)
        .assert()
        .success();

    assert_eq!(t.child("target").files(), vec!["CACHEDIR.TAG", "dev"]);
    assert_eq!(
        t.child("target/dev").files(),
        vec![".fingerprint", "hello.sierra.json", "incremental",]
    );
    let fingerprints = t.child("target/dev/.fingerprint").files();
    // We search the dir, as fingerprints will change with different temp dir, so we cannot hardcode
    // the name here.
    let core_component_id = fingerprints
        .iter()
        .find(|t| t.starts_with("core-"))
        .unwrap();
    assert_eq!(core_component_id.len(), 5 + 13); // 5 for "core-" and 13 for the hash
    let hello_component_id = fingerprints
        .iter()
        .find(|t| t.starts_with("hello-"))
        .unwrap();
    assert_eq!(hello_component_id.len(), 6 + 13); // 5 for "hello-" and 13 for the hash
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
    let core_component_digest = t
        .child(format!("target/dev/.fingerprint/{core_component_id}/core"))
        .read_to_string();
    assert_eq!(core_component_digest.len(), 13);
    let hello_component_digest = t
        .child(format!(
            "target/dev/.fingerprint/{hello_component_id}/hello"
        ))
        .read_to_string();
    assert_eq!(hello_component_digest.len(), 13);

    Scarb::quick_snapbox()
        .env("SCARB_CACHE", cache_dir.path())
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
    assert_eq!(
        t.child(format!("target/dev/.fingerprint/{core_component_id}/core"))
            .read_to_string(),
        core_component_digest
    );
    assert_eq!(
        t.child(format!(
            "target/dev/.fingerprint/{hello_component_id}/hello"
        ))
        .read_to_string(),
        hello_component_digest
    );
}

#[test]
fn deps_are_fingerprinted() {
    let cache_dir = TempDir::new().unwrap().child("c");
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
    ProjectBuilder::start().name("third").build(&third);

    Scarb::quick_snapbox()
        .env("SCARB_CACHE", cache_dir.path())
        .arg("build")
        .current_dir(&first)
        .assert()
        .success();

    let fingerprints = t.child("first/target/dev/.fingerprint").files();
    let component_id = fingerprints
        .iter()
        .find(|t| t.starts_with("first-"))
        .unwrap();
    let digest = t
        .child(format!(
            "first/target/dev/.fingerprint/{component_id}/first"
        ))
        .read_to_string();
    assert_eq!(digest.len(), 13);

    // Modify the third package.
    ProjectBuilder::start()
        .name("third")
        .version("2.0.0")
        .build(&third);

    Scarb::quick_snapbox()
        .env("SCARB_CACHE", cache_dir.path())
        .arg("build")
        .current_dir(&first)
        .assert()
        .success();

    assert_ne!(
        t.child(format!(
            "first/target/dev/.fingerprint/{component_id}/first"
        ))
        .read_to_string(),
        digest
    );
}
