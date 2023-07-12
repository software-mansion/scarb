use std::env;
use std::path::PathBuf;

/// Nextest-compatible variant of [`snapbox::cmd::cargo_bin()`].
pub fn cargo_bin(name: &str) -> PathBuf {
    env::var_os(format!("NEXTEST_BIN_EXE_{name}"))
        .or_else(|| env::var_os(format!("CARGO_BIN_EXE_{name}")))
        .map(PathBuf::from)
        .unwrap_or_else(|| snapbox::cmd::cargo_bin(name))
}

pub fn manifest_dir() -> PathBuf {
    env::var_os("CARGO_MANIFEST_DIR")
        .expect("CARGO_MANIFEST_DIR is not set")
        .into()
}
