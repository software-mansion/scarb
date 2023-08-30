use std::ffi::OsString;
use std::path::PathBuf;
use std::{env, iter, vec};

use assert_fs::prelude::*;
use assert_fs::TempDir;
use indoc::indoc;
use scarb::process::make_executable;

pub fn write_script(name: &str, script_source: &str, t: &TempDir) {
    let script = t.child(format!("scarb-{name}{}", env::consts::EXE_SUFFIX));
    script.write_str(script_source).unwrap();
    make_executable(script.path());
}

pub fn write_simple_hello_script(name: &str, t: &TempDir) {
    write_script(
        name,
        indoc! {r#"
            #!/usr/bin/env sh
            echo "Hello $@"
        "#},
        t,
    );
}

fn asdf_dir_paths() -> Vec<OsString> {
    vec![
        env::var_os("ASDF_DATA_DIR"),
        env::var_os("ASDF_DIR"),
        env::var_os("HOME").map(|home_var| PathBuf::from(home_var).join(".asdf").into()),
    ]
    .into_iter()
    .flatten()
    .collect()
}

fn os_path_without_asdf_dir() -> OsString {
    let os_path = env::var_os("PATH").unwrap();
    let asdf_dirs = asdf_dir_paths();

    let filtered_paths: Vec<PathBuf> = env::split_paths(&os_path)
        .filter(|path| !asdf_dirs.iter().any(|asdf_dir| path.starts_with(asdf_dir)))
        .collect();

    env::join_paths(filtered_paths).unwrap()
}

pub fn path_with_temp_dir(t: &TempDir) -> OsString {
    let script_path = iter::once(t.path().to_path_buf());
    let os_path = os_path_without_asdf_dir();
    let other_paths = env::split_paths(&os_path);
    env::join_paths(script_path.chain(other_paths)).unwrap()
}
