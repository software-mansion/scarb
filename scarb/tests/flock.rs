use std::io::{BufRead, BufReader};
use std::process::Stdio;
use std::sync::{Arc, Barrier};
use std::thread;

use assert_fs::fixture::PathChild;
use indoc::indoc;
use io_tee::TeeReader;
use ntest::timeout;

use scarb_test_support::command::Scarb;
use scarb_test_support::project_builder::ProjectBuilder;

#[test]
#[timeout(60_000)]
fn locking_build_artifacts() {
    let t = assert_fs::TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello")
        .version("0.1.0")
        .build(&t);

    let manifest = t.child("Scarb.toml");
    let config = Scarb::test_config(manifest);

    thread::scope(|s| {
        let ws = scarb::ops::read_workspace(config.manifest_path(), &config).unwrap();
        let lock = ws.target_dir().child(config.profile().to_string()).open_rw(
            "hello.sierra.json",
            "artifact",
            &config,
        );
        let barrier = Arc::new(Barrier::new(2));

        s.spawn({
            let barrier = barrier.clone();
            move || {
                barrier.wait();
                drop(lock);
            }
        });

        let mut proc = Scarb::from_config(&config)
            .std()
            .arg("build")
            .current_dir(&t)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();

        let mut stdout_acc = Vec::<u8>::new();
        let stdout = proc.stdout.take().unwrap();
        let stdout = TeeReader::new(stdout, &mut stdout_acc);
        let stdout = BufReader::new(stdout);
        for line in stdout.lines() {
            let line = line.unwrap();

            if line.contains("file lock") {
                barrier.wait();
            }
        }

        let ecode = proc.wait().unwrap();
        assert!(ecode.success());

        snapbox::assert_matches(
            indoc! {r#"
            [..] Compiling hello v0.1.0 ([..])
            [..]  Blocking waiting for file lock on output file
            [..]  Finished release target(s) in [..]
            "#},
            stdout_acc,
        );
    });
}

#[tokio::test(flavor = "multi_thread")]
#[timeout(60_000)]
async fn locking_package_cache() {
    let t = assert_fs::TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello")
        .version("0.1.0")
        .build(&t);

    let manifest = t.child("Scarb.toml");
    let config = Scarb::test_config(manifest);

    let lock = config.package_cache_lock().acquire_async().await;
    let barrier = Arc::new(Barrier::new(2));

    tokio::spawn({
        let barrier = barrier.clone();

        async move {
            barrier.wait();
            drop(lock);
        }
    });

    let mut proc = Scarb::from_config(&config)
        .std()
        .arg("fetch")
        .current_dir(&t)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();

    let mut stdout_acc = Vec::<u8>::new();
    let stdout = proc.stdout.take().unwrap();
    let stdout = TeeReader::new(stdout, &mut stdout_acc);
    let stdout = BufReader::new(stdout);
    for line in stdout.lines() {
        let line = line.unwrap();

        if line.contains("file lock") {
            barrier.wait();
        }
    }

    let ecode = proc.wait().unwrap();
    assert!(ecode.success());

    snapbox::assert_matches(
        indoc! {r#"
        [..]  Blocking waiting for file lock on package cache
        "#},
        stdout_acc,
    );
}
