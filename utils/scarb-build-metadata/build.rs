use cargo_metadata::camino::Utf8PathBuf;
use std::path::{Path, PathBuf};
use std::process::Command;

use cargo_metadata::{MetadataCommand, Package};

fn main() {
    commit_info();
    cairo_version();
}

fn commit_info() {
    if !Path::new("../../.git").exists() {
        return;
    }
    println!("cargo:rerun-if-changed=../../.git/index");
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
    let cargo_lock = find_cargo_lock();
    println!("cargo:rerun-if-changed={}", cargo_lock.display());

    let metadata = MetadataCommand::new()
        .manifest_path("../../scarb/Cargo.toml")
        .verbose(true)
        .exec()
        .expect("Failed to execute cargo metadata");

    let resolve = metadata
        .resolve
        .expect("Expected metadata resolve to be present.");

    let root = resolve
        .root
        .expect("Expected metadata resolve root to be present.");
    assert!(
        // The first condition for Rust >= 1.77
        // (After the PackageId spec stabilization)
        // The second condition for Rust < 1.77
        root.repr.contains("scarb#") || root.repr.starts_with("scarb "),
        "Expected metadata resolve root to be `scarb`."
    );

    let scarb_node = resolve.nodes.iter().find(|node| node.id == root).unwrap();
    let compiler_dep = scarb_node
        .deps
        .iter()
        .find(|dep| dep.name == "cairo_lang_compiler")
        .unwrap();
    let compiler_package = metadata
        .packages
        .iter()
        .find(|pkg| pkg.id == compiler_dep.pkg)
        .unwrap();
    let version = compiler_package.version.to_string();
    println!("cargo:rustc-env=SCARB_CAIRO_VERSION={version}");

    if let Some(corelib_local_path) =
        find_corelib_local_path(compiler_package).map(|p| p.to_string())
    {
        println!("cargo:rustc-env=SCARB_CORELIB_LOCAL_PATH={corelib_local_path}");
    }

    let mut rev = format!("refs/tags/v{version}");
    if let Some(source) = &compiler_package.source {
        let source = source.to_string();
        if source.starts_with("git+") {
            if let Some((_, commit)) = source.split_once('#') {
                println!("cargo:rustc-env=SCARB_CAIRO_COMMIT_HASH={commit}");
                let mut short_commit = commit.to_string();
                short_commit.truncate(9);
                println!("cargo:rustc-env=SCARB_CAIRO_SHORT_COMMIT_HASH={short_commit}");
                rev = commit.to_string();
            }
        }
    }
    println!("cargo:rustc-env=SCARB_CAIRO_COMMIT_REV={rev}");
}

/// Find corelib in local cargo cache.
///
/// This function lookups `cairo-lang-compiler` crate in local cargo cache.
/// This cache should be populated by Cargo, on `cargo metadata` call.
/// It relies on manifest path provided by cargo metadata, and searches parent directories.
/// If the crate is downloaded from the registry, the corelib will not be included.
/// If the crate is downloaded as git or path dependency, the corelib should be present.
fn find_corelib_local_path(compiler_package: &Package) -> Option<Utf8PathBuf> {
    // The following logic follows Cairo repository layout.
    // Starts with `cairo-lang-compiler` crate's manifest path.
    compiler_package
        .manifest_path
        // Crate root directory.
        .parent()
        // The `crates` directory from Cairo repository.
        .and_then(|p| p.parent())
        // The Cairo repository root.
        .and_then(|p| p.parent())
        // Corelib should be present in Cairo compiler repository root.
        .map(|p| p.join("corelib"))
        // Ensure path exists
        .and_then(|p| if p.exists() { Some(p) } else { None })
    // Note, that for registry source, we do not get whole Cairo repository in cache.
    // Thus the corelib will not be found - only the crate is downloaded.
}

fn find_cargo_lock() -> PathBuf {
    let in_workspace = PathBuf::from("../../Cargo.lock");
    if in_workspace.exists() {
        return in_workspace;
    }

    let in_package = PathBuf::from("Cargo.lock");
    if in_package.exists() {
        return in_package;
    }

    panic!(
        "Couldn't find Cargo.lock of this package. \
        Something's wrong with build execution environment."
    )
}
