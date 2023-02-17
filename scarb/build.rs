use std::fs;
use std::path::Path;
use std::process::Command;
use std::str::FromStr;

fn main() {
    commit_info();
    cairo_version();
}

fn commit_info() {
    if !Path::new("../.git").exists() {
        return;
    }
    println!("cargo:rerun-if-changed=../.git/index");
    let output = match Command::new("git")
        .arg("log")
        .arg("-1")
        .arg("--date=short")
        .arg("--format=%H %h %cd")
        .arg("--abbrev=9")
        .current_dir("..")
        .output()
    {
        Ok(output) if output.status.success() => output,
        _ => return,
    };
    let stdout = String::from_utf8(output.stdout).unwrap();
    let mut parts = stdout.split_whitespace();
    let mut next = || parts.next().unwrap();
    println!("cargo:rustc-env=SCARB_COMMIT_HASH={}", next());
    println!("cargo:rustc-env=SCARB_COMMIT_SHORT_HASH={}", next());
    println!("cargo:rustc-env=SCARB_COMMIT_DATE={}", next())
}

fn cairo_version() {
    println!("cargo:rerun-if-changed=../Cargo.lock");
    let Ok(lock) = fs::read_to_string("../Cargo.lock") else { return };
    let Ok(lock) = toml_edit::Document::from_str(&lock) else { return };
    let Some(lock) = lock["package"].as_array_of_tables() else { return };
    let Some(cairo_lock) = lock.into_iter().find(|t| {
        t["name"].as_value().and_then(|v| v.as_str()).unwrap_or_default() == "cairo-lang-compiler"
    }) else {
        return;
    };
    let Some(version) = cairo_lock["version"].as_str() else { return };
    println!("cargo:rustc-env=SCARB_CAIRO_VERSION={version}");
    if let Some(source) = cairo_lock["source"].as_str() {
        if source.starts_with("git+") {
            if let Some((_, commit)) = source.split_once('#') {
                println!("cargo:rustc-env=SCARB_CAIRO_COMMIT_HASH={commit}");
            }
        }
    }
}
