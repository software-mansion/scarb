use std::ffi::OsString;
use std::{env, iter};

use assert_fs::prelude::*;
use assert_fs::TempDir;
use scarb::process::make_executable;

pub fn write_script(name: &str, script_source: &str, t: &TempDir) {
    let script = t.child(format!("scarb-{name}{}", env::consts::EXE_SUFFIX));
    script.write_str(script_source).unwrap();
    make_executable(script.path());
}

pub fn path_with_temp_dir(t: &TempDir) -> OsString {
    let script_path = iter::once(t.path().to_path_buf());
    let os_path = env::var_os("PATH").unwrap();
    let other_paths = env::split_paths(&os_path);
    env::join_paths(script_path.chain(other_paths)).unwrap()
}
