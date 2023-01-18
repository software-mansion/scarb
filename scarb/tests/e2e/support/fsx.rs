use assert_fs::fixture::ChildPath;
use assert_fs::TempDir;
use camino::Utf8Path;

pub use internal_fsx::{PathBufUtf8Ext, PathUtf8Ext};

#[allow(unused)]
#[path = "../../../src/internal/fsx.rs"]
mod internal_fsx;

pub trait AssertFsUtf8Ext {
    fn utf8_path(&self) -> &Utf8Path;
}

impl AssertFsUtf8Ext for TempDir {
    fn utf8_path(&self) -> &Utf8Path {
        self.path().try_as_utf8().unwrap()
    }
}

impl AssertFsUtf8Ext for ChildPath {
    fn utf8_path(&self) -> &Utf8Path {
        self.path().try_as_utf8().unwrap()
    }
}
