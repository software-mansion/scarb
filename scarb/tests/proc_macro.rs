use assert_fs::TempDir;
use scarb_test_support::command::Scarb;
use scarb_test_support::project_builder::{Dep, DepBuilder, ProjectBuilder};
use std::path::PathBuf;

#[test]
fn compile_simple() {
    let proc_macro_stub_path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fixtures/proc-macro-stub/");
    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("hello")
        .dep(
            "proc_macro_stub",
            Dep.path(proc_macro_stub_path.as_path().to_string_lossy().to_string()),
        )
        .build(&t);

    Scarb::quick_snapbox()
        .arg("build")
        .current_dir(&t)
        .assert()
        .success();
}
