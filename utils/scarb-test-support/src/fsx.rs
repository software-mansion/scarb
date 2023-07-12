use std::fs::File;
use std::io::BufReader;

use assert_fs::fixture::ChildPath;
use assert_fs::TempDir;
use camino::Utf8Path;
use itertools::Itertools;
use serde::de::DeserializeOwned;

pub use internal_fsx::{PathBufUtf8Ext, PathUtf8Ext};

#[allow(unused)]
#[path = "../../../scarb/src/internal/fsx.rs"]
mod internal_fsx;

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
    fn files(&self) -> Vec<String>;
    fn assert_is_json<T: DeserializeOwned>(&self) -> T;
}

impl ChildPathEx for ChildPath {
    fn files(&self) -> Vec<String> {
        self.read_dir()
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into())
            .sorted()
            .collect()
    }

    fn assert_is_json<T: DeserializeOwned>(&self) -> T {
        let file = File::open(self.path()).unwrap();
        let reader = BufReader::new(file);
        serde_json::from_reader(reader).unwrap()
    }
}
