use std::fs;
use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf, MAIN_SEPARATOR_STR};

use assert_fs::fixture::ChildPath;
use assert_fs::TempDir;
use camino::Utf8Path;
use itertools::Itertools;
use serde::de::DeserializeOwned;

pub use internal_fsx::{canonicalize, canonicalize_utf8, PathBufUtf8Ext, PathUtf8Ext};

#[allow(unused)]
#[path = "../../../scarb/src/internal/fsx.rs"]
mod internal_fsx;

#[cfg(unix)]
pub fn make_executable(path: &Path) {
    use std::os::unix::prelude::*;
    let mut perms = fs::metadata(path).unwrap().permissions();
    perms.set_mode(perms.mode() | 0o700);
    fs::set_permissions(path, perms).unwrap();
}

#[cfg(windows)]
pub fn make_executable(_path: &Path) {}

pub trait AssertFsUtf8Ext {
    fn utf8_path(&self) -> &Utf8Path;
}

impl AssertFsUtf8Ext for TempDir {
    fn utf8_path(&self) -> &Utf8Path {
        self.path().try_as_utf8().unwrap()
    }
}

impl AssertFsUtf8Ext for &TempDir {
    fn utf8_path(&self) -> &Utf8Path {
        self.path().try_as_utf8().unwrap()
    }
}

impl AssertFsUtf8Ext for ChildPath {
    fn utf8_path(&self) -> &Utf8Path {
        self.path().try_as_utf8().unwrap()
    }
}

impl AssertFsUtf8Ext for &ChildPath {
    fn utf8_path(&self) -> &Utf8Path {
        self.path().try_as_utf8().unwrap()
    }
}

pub trait ChildPathEx {
    fn read_to_string(&self) -> String;
    fn files(&self) -> Vec<String>;
    fn tree(&self) -> String;
    fn assert_is_json<T: DeserializeOwned>(&self) -> T;
    fn assert_is_toml_document(&self) -> toml_edit::Document;
}

impl ChildPathEx for ChildPath {
    fn read_to_string(&self) -> String {
        fs::read_to_string(self.path()).unwrap()
    }

    fn files(&self) -> Vec<String> {
        self.read_dir()
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into())
            .sorted()
            .collect()
    }

    fn tree(&self) -> String {
        fn visit(path: &ChildPath, paths: &mut Vec<PathBuf>) {
            paths.push(path.path().to_owned());
            if path.is_dir() {
                for entry in path.read_dir().unwrap() {
                    let entry = entry.unwrap();
                    let entry = ChildPath::new(entry.path());
                    visit(&entry, paths);
                }
            }
        }

        let mut paths = Vec::with_capacity(32);
        visit(self, &mut paths);
        paths.sort();

        let mut out = String::with_capacity(paths.len() * 32);
        for path in paths {
            let is_dir = path.is_dir();
            let path = path.strip_prefix(self).unwrap();
            let mut components = path.components();
            let Some(file_name) = components.next_back() else {
                continue;
            };
            for _ in components {
                out += ". ";
            }
            out += file_name.as_os_str().to_string_lossy().as_ref();
            if is_dir {
                out += "/";
            }
            out += "\n";
        }
        out
    }

    fn assert_is_json<T: DeserializeOwned>(&self) -> T {
        let file = File::open(self.path()).unwrap();
        let reader = BufReader::new(file);
        serde_json::from_reader(reader).unwrap()
    }

    fn assert_is_toml_document(&self) -> toml_edit::Document {
        self.read_to_string().parse().unwrap()
    }
}

/// Convert all UNIX-style paths in a string to platform native.
///
/// This method is doing dump pattern search & replace, it might replace unexpected parts of the
/// input string. Use with caution.
pub fn unix_paths_to_os_lossy(text: &str) -> String {
    if cfg!(unix) {
        text.to_string()
    } else {
        text.replace('/', MAIN_SEPARATOR_STR)
    }
}
