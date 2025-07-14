use assert_fs::TempDir;
use assert_fs::prelude::{FileWriteStr, PathChild};
use scarb_test_support::command::{Scarb, ScarbSnapboxExt};
use scarb_test_support::fsx::ChildPathEx;
use scarb_test_support::project_builder::ProjectBuilder;

#[test]
fn incremental_artifacts_emitted() {
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

    let component_id = |name: &str| {
        fingerprints()
            .iter()
            .find(|t| t.starts_with(&format!("{name}-")))
            .unwrap()
            .to_string()
    };
    let digest = |component_id: &str| {
        let (name, _) = component_id.split_once("-").unwrap();
        t.child(format!(
            "first/target/dev/.fingerprint/{component_id}/{name}"
        ))
        .read_to_string()
    };

    let first_component_id = component_id("first");
    let first_digest = digest(first_component_id.as_str());
    assert_eq!(first_digest.len(), 13);

    let second_component_id = component_id("second");
    let second_digest = digest(second_component_id.as_str());
    assert_eq!(second_digest.len(), 13);

    let fourth_component_id = component_id("fourth");
    let fourth_digest = digest(fourth_component_id.as_str());
    assert_eq!(fourth_digest.len(), 13);

    // Modify the third package.
    ProjectBuilder::start()
        .name("third")
        .version("2.0.0")
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

    // Note we have changed the version of the third.
    // Since second depends on third, and direct deps package ids are part of the fingerprint id,
    // second will change its id.
    assert_eq!(fingerprints().len(), 8); // core, first, second, second, third, third, fourth, fifth
    assert_eq!(digest(second_component_id.as_str()), second_digest);
    let new_second_component_id = fingerprints()
        .iter()
        .find(|t| t.starts_with("second-") && t.as_str() != second_component_id)
        .unwrap()
        .to_string();
    assert_ne!(digest(new_second_component_id.as_str()), second_digest);
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

    let component_id = |name: &str| {
        fingerprints()
            .iter()
            .find(|t| t.starts_with(&format!("{name}-")))
            .unwrap()
            .to_string()
    };
    let digest = |component_id: &str| {
        let (name, _) = component_id.split_once("-").unwrap();
        target_dir
            .child(format!("dev/.fingerprint/{component_id}/{name}"))
            .read_to_string()
    };

    let first_component_id = component_id("first");
    let first_digest = digest(first_component_id.as_str());
    assert_eq!(first_digest.len(), 13);

    let second_component_id = component_id("second");
    let second_digest = digest(second_component_id.as_str());
    assert_eq!(second_digest.len(), 13);

    let third_component_id = component_id("third");
    let third_digest = digest(third_component_id.as_str());
    assert_eq!(third_digest.len(), 13);

    let fourth_component_id = component_id("fourth");
    let fourth_digest = digest(fourth_component_id.as_str());
    assert_eq!(fourth_digest.len(), 13);

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
