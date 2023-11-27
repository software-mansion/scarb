use assert_fs::prelude::*;
use assert_fs::TempDir;
use indoc::indoc;
use toml_edit::value;

use scarb_test_support::command::Scarb;
use scarb_test_support::fsx::ChildPathEx;
use scarb_test_support::gitx;
use scarb_test_support::project_builder::{Dep, DepBuilder, ProjectBuilder};
use scarb_test_support::registry::local::LocalRegistry;

#[test]
fn checksum_changed_upstream() {
    let mut registry = LocalRegistry::create();
    registry.publish(|t| {
        ProjectBuilder::start()
            .name("bar")
            .version("1.0.0")
            .lib_cairo(r#"fn f() -> felt252 { 0 }"#)
            .build(t);
    });

    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("foo")
        .version("0.1.0")
        .dep("bar", Dep.version("1.0.0").registry(&registry))
        .lib_cairo(r#"fn f() -> felt252 { bar::f() }"#)
        .build(&t);

    Scarb::quick_snapbox()
        .arg("fetch")
        .current_dir(&t)
        .assert()
        .success();

    let expected_lockfile = t.child("Scarb.lock").read_to_string();

    // Now, let's redeploy the same package, but with a different source → different checksum.
    registry.publish(|t| {
        ProjectBuilder::start()
            .name("bar")
            .version("1.0.0")
            .lib_cairo(r#"fn f() -> felt252 { 1234 }"#)
            .build(t);
    });

    Scarb::quick_snapbox()
        .arg("fetch")
        .current_dir(&t)
        .assert()
        .failure()
        .stdout_matches(indoc! {r#"
        error: checksum for `bar v1.0.0 (registry+file://[..])` changed between lock files

        this could be indicative of a few possible errors:

            * the lock file is corrupt
            * a replacement source in use (e.g. a mirror) returned a different checksum
            * the source itself may be corrupt in one way or another

        unable to verify that `bar v1.0.0 (registry+file://[..])` is the same as when the lockfile was generated
        "#});

    // Let's verify that the lockfile was not modified.
    let actual_lockfile = t.child("Scarb.lock").read_to_string();
    assert_eq!(expected_lockfile, actual_lockfile);
}

#[test]
fn checksum_locked_for_unexpected_source() {
    let bar = gitx::new("bar", |t| {
        ProjectBuilder::start()
            .name("bar")
            .version("1.0.0")
            .build(&t);
    });

    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("foo")
        .version("0.1.0")
        .dep("bar", &bar)
        .build(&t);

    // Let's generate a valid Scarb.lock and then corrupt it, by adding a `checksum` where it is
    // not supposed to be. Git dependencies are not expected to ever have checksums.
    Scarb::quick_snapbox()
        .arg("fetch")
        .current_dir(&t)
        .assert()
        .success();

    let mut lockfile = t.child("Scarb.lock").assert_is_toml_document();
    lockfile
        .get_mut("package")
        .unwrap()
        .as_array_of_tables_mut()
        .unwrap()
        .iter_mut()
        .find(|t| t["name"].as_str().unwrap() == "bar")
        .unwrap()
        .insert(
            "checksum",
            value("sha256:b62fc4b9bfbd9310a47d2e595d2c8f468354266be0827aeea9b465d9984908de"),
        );
    t.child("Scarb.lock")
        .write_str(&lockfile.to_string())
        .unwrap();

    Scarb::quick_snapbox()
        .arg("fetch")
        .current_dir(&t)
        .assert()
        .failure()
        .stdout_matches(indoc! {r#"
        [..] Updating git repository [..]
        error: checksum for `bar v1.0.0 ([..])` could not be calculated, but a checksum is listed in the existing lock file

        this could be indicative of a few possible situations:

            * the source `[..]` supports checksums, but was replaced with one that does not
            * the lock file is corrupt

        unable to verify that `bar v1.0.0 ([..])` is the same as when the lockfile was generated
        "#});
}

#[test]
fn unlisted_checksum_for_source_supporting_it() {
    let mut registry = LocalRegistry::create();
    registry.publish(|t| {
        ProjectBuilder::start()
            .name("bar")
            .version("1.0.0")
            .build(t);
    });

    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("foo")
        .version("0.1.0")
        .dep("bar", Dep.version("1.0.0").registry(&registry))
        .build(&t);

    // Let's generate a valid Scarb.lock and then corrupt it, by removing `checksum` fields
    // everywhere applicable.
    Scarb::quick_snapbox()
        .arg("fetch")
        .current_dir(&t)
        .assert()
        .success();

    let mut lockfile = t.child("Scarb.lock").assert_is_toml_document();
    lockfile
        .get_mut("package")
        .unwrap()
        .as_array_of_tables_mut()
        .unwrap()
        .iter_mut()
        .for_each(|t| {
            t.remove("checksum");
        });
    t.child("Scarb.lock")
        .write_str(&lockfile.to_string())
        .unwrap();

    Scarb::quick_snapbox()
        .arg("fetch")
        .current_dir(&t)
        .assert()
        .failure()
        .stdout_matches(indoc! {r#"
        error: checksum for `bar v1.0.0 ([..])` was not previously calculated, but now it could be

        this could be indicative of a few possible situations:

            * the source `[..]` did not previously support checksums, but was replaced with one that does
            * newer Scarb implementations know how to checksum this source, but this older implementation does not
            * the lock file is corrupt
        "#});
}
