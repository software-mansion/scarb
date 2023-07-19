use std::fs::File;
use std::path::PathBuf;
use std::process::Command;
use std::{env, fs, io};

use zip::ZipArchive;

use scarb_build_metadata::CAIRO_COMMIT_REV;

fn main() {
    download_core(CAIRO_COMMIT_REV);
}

fn is_docs_rs() -> bool {
    env::var("DOCS_RS").is_ok()
}

fn download_core(rev: &str) {
    println!("cargo:rerun-if-env-changed=CAIRO_ARCHIVE");
    let out_dir = env::var("OUT_DIR").unwrap();
    if is_docs_rs() {
        eprintln!("Docs.rs build detected. Skipping corelib download.");
        let core_stub_path = PathBuf::from_iter([&out_dir, "core-stub"]);
        fs::create_dir_all(&core_stub_path).unwrap();
        println!(
            "cargo:rustc-env=SCARB_CORE_PATH={}",
            core_stub_path.display()
        );
        return;
    }

    let core_path = PathBuf::from_iter([&out_dir, &format!("core-{}", ident(rev))]);
    if !core_path.is_dir() {
        let cairo_zip = PathBuf::from_iter([&out_dir, "cairo.zip"]);

        if let Ok(cairo_archive) = env::var("CAIRO_ARCHIVE") {
            // Copy archive to `cairo_zip`, without keeping file attributes.
            eprintln!("Copying Cairo archive from `CAIRO_ARCHIVE={cairo_archive}`.");
            let mut src = File::open(&cairo_archive).unwrap();
            let mut dst = File::create(&cairo_zip).unwrap();
            io::copy(&mut src, &mut dst).unwrap();
        } else {
            let url = format!("https://github.com/starkware-libs/cairo/archive/{rev}.zip");
            let mut curl = Command::new("curl");
            curl.args(["--proto", "=https", "--tlsv1.2", "-fL"]);
            curl.arg("-o");
            curl.arg(&cairo_zip);
            curl.arg(&url);
            eprintln!("{curl:?}");
            let curl_exit = curl.status().expect("Failed to start curl");
            if !curl_exit.success() {
                panic!("Failed to download {url} with curl")
            }
        }

        fs::create_dir_all(&core_path).unwrap();
        let cairo_file = File::open(cairo_zip).unwrap();
        let mut cairo_archive = ZipArchive::new(cairo_file).unwrap();
        for i in 0..cairo_archive.len() {
            let mut input = cairo_archive.by_index(i).unwrap();

            if input.name().ends_with('/') {
                continue;
            }

            let path = input.enclosed_name().unwrap();

            let path = PathBuf::from_iter(path.components().skip(1));
            let Ok(path) = path.strip_prefix("corelib") else {
                continue;
            };

            let path = core_path.join(path);

            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).unwrap();
            }

            let mut output = File::create(path).unwrap();
            io::copy(&mut input, &mut output).unwrap();
        }
    }

    println!("cargo:rustc-env=SCARB_CORE_PATH={}", core_path.display());
}

fn ident(id: &str) -> String {
    let mut ident = String::with_capacity(id.len());
    for ch in id.chars() {
        ident.push(if ch.is_ascii_alphanumeric() { ch } else { '_' })
    }
    ident
}
