use std::thread;
use std::time::Duration;

use assert_fs::fixture::{FileWriteStr, PathChild};
use indoc::indoc;

use scarb::core::Config;
use scarb::dirs::AppDirs;
use scarb::ui::{OutputFormat, Ui};

use crate::support::command::scarb_command;
use crate::support::fsx::AssertFsUtf8Ext;

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

    let config = Config::init(
        manifest.utf8_path().to_path_buf(),
        AppDirs::std().unwrap(),
        Ui::new(OutputFormat::Text),
    )
    .unwrap();

    let lock = config
        .target_dir()
        .child("release")
        .open_rw("hello.sierra", "artifact", &config);

    thread::scope(|s| {
        s.spawn(|| {
            thread::sleep(Duration::from_secs(2));
            drop(lock);
        });

        scarb_command()
            .arg("build")
            .current_dir(&t)
            .timeout(Duration::from_secs(5))
            .assert()
            .success()
            .stdout_matches(indoc! {r#"
                [..] Compiling hello v0.1.0 ([..])
                [..]  Blocking waiting for file lock on output file
                [..]  Finished release target(s) in [..]
            "#});
    });
}
