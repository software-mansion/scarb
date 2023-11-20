use assert_fs::prelude::*;
use assert_fs::TempDir;
use indoc::indoc;

use scarb_test_support::command::Scarb;
use scarb_test_support::project_builder::{Dep, DepBuilder, ProjectBuilder};
use scarb_test_support::registry::local::LocalRegistry;

#[test]
#[cfg_attr(
    not(target_family = "unix"),
    ignore = "registry test failing on windows"
)]
fn checksum_mismatch() {
    let mut registry = LocalRegistry::create();
    registry.publish(|t| {
        ProjectBuilder::start()
            .name("bar")
            .version("1.0.0")
            .lib_cairo(r#"fn f() -> felt252 { 0 }"#)
            .build(t);
    });

    registry
        .t
        .child("bar-1.0.0.tar.zst")
        .write_str(
            "This is a sequence of bytes that is definitely not a valid tar nor zst. \
            This way, we verify that Scarb is not even attempting to read/interpret \
            archives before verifying checksums.",
        )
        .unwrap();

    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("foo")
        .version("0.1.0")
        .dep("bar", Dep.version("1").registry(&registry))
        .lib_cairo(r#"fn f() -> felt252 { bar::f() }"#)
        .build(&t);

    // FIXME(mkaput): Why are verbose statuses not appearing here?
    Scarb::quick_snapbox()
        .arg("fetch")
        .current_dir(&t)
        .assert()
        .failure()
        .stdout_matches(indoc! {r#"
        error: failed to download package: bar v1.0.0 (registry+file://[..])

        Caused by:
            failed to verify the checksum of downloaded archive
        "#});
}
