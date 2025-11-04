use std::env;
use std::path::PathBuf;

/// Nextest-compatible variant of [`snapbox::cmd::cargo_bin()`].
pub fn cargo_bin(name: &str) -> PathBuf {
    env::var_os(format!("NEXTEST_BIN_EXE_{name}"))
        .or_else(|| env::var_os(format!("CARGO_BIN_EXE_{name}")))
        .map(PathBuf::from)
        .unwrap_or_else(|| lookup_cargo_bin(name))
}

/// Look up the path to a cargo-built binary within an integration test.
///
/// Makes assumptions about Cargo.
fn lookup_cargo_bin(name: &str) -> PathBuf {
    let file_name = format!("{}{}", name, std::env::consts::EXE_SUFFIX);
    let target_dir = target_dir();
    target_dir.join(file_name)
}

// Adapted from
// https://github.com/rust-lang/cargo/blob/485670b3983b52289a2f353d589c57fae2f60f82/tests/testsuite/support/mod.rs#L507
fn target_dir() -> PathBuf {
    env::current_exe()
        .ok()
        .map(|mut path| {
            path.pop();
            if path.ends_with("deps") {
                path.pop();
            }
            path
        })
        .unwrap()
}

pub fn manifest_dir() -> PathBuf {
    env::var_os("CARGO_MANIFEST_DIR")
        .expect("CARGO_MANIFEST_DIR is not set")
        .into()
}
