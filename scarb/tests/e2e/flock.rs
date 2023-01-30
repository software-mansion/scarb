use std::thread;
use std::time::Duration;

use assert_fs::fixture::{FileWriteStr, PathChild};
use indoc::indoc;

use crate::support::command::Scarb;

#[test]
fn locking_build_artifacts() {
    let t = assert_fs::TempDir::new().unwrap();
    let manifest = t.child("Scarb.toml");
    manifest
        .write_str(
            r#"
            [package]
            name = "hello"
            version = "0.1.0"
            "#,
        )
        .unwrap();
    t.child("src/lib.cairo")
        .write_str(r#"fn f() -> felt { 42 }"#)
        .unwrap();

    let config = Scarb::test_config(&manifest);

    let lock = config
        .target_dir()
        .child("release")
        .open_rw("hello.sierra", "artifact", &config);

    thread::scope(|s| {
        s.spawn(|| {
            thread::sleep(Duration::from_secs(4));
            drop(lock);
        });

        Scarb::from_config(&config)
            .snapbox()
            .arg("build")
            .current_dir(&t)
            .timeout(Duration::from_secs(10))
            .assert()
            .success()
            .stdout_matches(indoc! {r#"
                [..] Compiling hello v0.1.0 ([..])
                [..]  Blocking waiting for file lock on output file
                [..]  Finished release target(s) in [..]
            "#});
    });
}

#[test]
fn locking_package_cache() {
    let t = assert_fs::TempDir::new().unwrap();
    let manifest = t.child("Scarb.toml");
    manifest
        .write_str(
            r#"
            [package]
            name = "hello"
            version = "0.1.0"
            "#,
        )
        .unwrap();
    t.child("src/lib.cairo")
        .write_str(r#"fn f() -> felt { 42 }"#)
        .unwrap();

    let config = Scarb::test_config(&manifest);

    let lock = config.package_cache_lock().acquire();

    thread::scope(|s| {
        s.spawn(|| {
            thread::sleep(Duration::from_secs(4));
            drop(lock);
        });

        Scarb::from_config(&config)
            .snapbox()
            .arg("build")
            .current_dir(&t)
            .timeout(Duration::from_secs(10))
            .assert()
            .success()
            .stdout_matches(indoc! {r#"
                [..]  Blocking waiting for file lock on package cache
                [..] Compiling hello v0.1.0 ([..])
                [..]  Finished release target(s) in [..]
            "#});
    });
}
