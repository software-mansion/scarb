use std::process::Stdio;
use std::time::{Duration, Instant};

use assert_fs::{TempDir, prelude::*};
use ntest::timeout;
use snapbox::Data;

use scarb_test_support::command::Scarb;
use scarb_test_support::project_builder::ProjectBuilder;

#[test]
fn simple_clean() {
    let t = TempDir::new().unwrap();
    ProjectBuilder::start().build(&t);
    let cache_dir = TempDir::new().unwrap();

    Scarb::new()
        .cache(cache_dir.path())
        .command()
        .arg("fetch")
        .current_dir(&t)
        .assert()
        .success();
    cache_dir.assert(predicates::path::is_dir());

    Scarb::new()
        .cache(cache_dir.path())
        .command()
        .arg("cache")
        .arg("clean")
        .current_dir(&t)
        .assert()
        .success();
    // After `cache clean`, the cache dir survives but contains only the
    // package cache lock file. See `ops::cache::PACKAGE_CACHE_LOCK_FILENAME`
    // for why the lock file is preserved across clean.
    cache_dir.assert(predicates::path::is_dir());
    let entries: Vec<_> = std::fs::read_dir(cache_dir.path())
        .unwrap()
        .map(|e| e.unwrap().file_name())
        .collect();
    assert_eq!(entries, [".package-cache.lock"]);
}

#[test]
fn path_print() {
    let t = TempDir::new().unwrap();
    ProjectBuilder::start().build(&t);
    let cache_dir = TempDir::new().unwrap();

    Scarb::new()
        .cache(cache_dir.path())
        .command()
        .arg("cache")
        .arg("path")
        .current_dir(&t)
        .assert()
        .stdout_eq(Data::from(format!("{}\n", cache_dir.path().display())).raw())
        .success();
    cache_dir.assert(predicates::path::is_dir());
}

// Regression test for the lock-file race in `ops::cache::cache_clean`:
//   1. clean acquires `flock(EX)` on `<cache>/.package-cache.lock`,
//   2. `remove_dir_all(<cache>)` unlinks the lock file,
//   3. a concurrent `package_cache_lock().acquire_async()` recreates the
//      cache dir, opens a brand-new lock file at the same path (different
//      inode), and `flock(EX)` succeeds immediately — `flock` is per-inode,
//      so it does not conflict with the still-open, now-unlinked old inode.
//
// Without a fix, the two scarb processes both believe they hold the
// exclusive package cache lock at the same time.
//
// This test makes the race deterministic via the
// `SCARB_INTERNAL_CACHE_CLEAN_PAUSE` hook in `cache_clean`: clean is paused
// while still holding `_lock` and after `remove_dir_all` has completed, the
// test then runs `scarb fetch` and signals clean to resume.
//
// Correct behavior: while clean holds the package cache lock, `scarb fetch`
// must block on lock acquisition and emit `flock.rs`'s "Blocking waiting
// for file lock" message. The test asserts that, and therefore fails until
// the lock is kept on a stable inode across `cache clean` (e.g. moved out
// of the cache dir).
#[cfg(unix)]
#[test]
#[timeout(60_000)]
fn cache_clean_race_with_concurrent_lock_acquisition() {
    let cache_dir = TempDir::new().unwrap();
    let config_dir = TempDir::new().unwrap();
    let sync_dir = TempDir::new().unwrap();
    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello")
        .version("0.1.0")
        .build(&t);

    // cache_clean only does work when the cache dir already exists.
    std::fs::create_dir_all(cache_dir.path()).unwrap();

    let prefix = sync_dir.child("clean-pause");
    let ready_path = sync_dir.child("clean-pause.ready");
    let go_path = sync_dir.child("clean-pause.go");

    let mut clean = Scarb::new()
        .cache(cache_dir.path())
        .config(config_dir.path())
        .std()
        .arg("cache")
        .arg("clean")
        .env("SCARB_INTERNAL_CACHE_CLEAN_PAUSE", prefix.path())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();

    // Wait until cache_clean has acquired the lock and finished remove_dir_all.
    let deadline = Instant::now() + Duration::from_secs(30);
    while !ready_path.path().exists() {
        if Instant::now() > deadline {
            let _ = clean.kill();
            panic!("scarb cache clean never reached the pause sentinel");
        }
        std::thread::sleep(Duration::from_millis(20));
    }

    // Clean is now paused mid-flight: cache dir + lock file unlinked, but it
    // still holds the flock on the original inode. Spawn fetch — under a
    // correct implementation it must block on lock acquisition until clean
    // releases.
    let fetch = Scarb::new()
        .cache(cache_dir.path())
        .config(config_dir.path())
        .std()
        .arg("fetch")
        .current_dir(&t)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();

    // Give fetch a moment to reach `acquire_async` and either block (correct)
    // or grab a brand-new-inode lock (buggy). Then release clean so fetch
    // can complete in either case.
    std::thread::sleep(Duration::from_millis(500));
    std::fs::write(go_path.path(), "").unwrap();

    let fetch_output = fetch.wait_with_output().unwrap();
    let clean_output = clean.wait_with_output().unwrap();

    let fetch_stdout = String::from_utf8_lossy(&fetch_output.stdout);
    let fetch_stderr = String::from_utf8_lossy(&fetch_output.stderr);
    let clean_stdout = String::from_utf8_lossy(&clean_output.stdout);
    let clean_stderr = String::from_utf8_lossy(&clean_output.stderr);

    assert!(
        clean_output.status.success(),
        "scarb cache clean failed\nstdout: {clean_stdout}\nstderr: {clean_stderr}",
    );
    assert!(
        fetch_output.status.success(),
        "scarb fetch failed\nstdout: {fetch_stdout}\nstderr: {fetch_stderr}",
    );
    assert!(
        fetch_stdout.contains("Blocking waiting for file lock"),
        "scarb fetch did NOT block on the package cache lock while \
         `scarb cache clean` was holding it — both processes acquired what \
         they each consider an exclusive lock at the same time. \
         fetch stdout: {fetch_stdout}",
    );
}
