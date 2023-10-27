use std::fmt;

use assert_fs::TempDir;
use url::Url;

use crate::command::Scarb;

pub struct LocalRegistry {
    pub t: TempDir,
    pub url: String,
}

impl LocalRegistry {
    pub fn create() -> Self {
        let t = TempDir::new().unwrap();
        let url = Url::from_directory_path(&t).unwrap().to_string();
        Self { t, url }
    }

    pub fn publish(&mut self, f: impl FnOnce(&TempDir)) -> &mut Self {
        let t = TempDir::new().unwrap();
        f(&t);
        Scarb::quick_snapbox()
            .arg("publish")
            .arg("--index")
            .arg(&self.url)
            .current_dir(&t)
            .assert()
            .success();
        self
    }
}

impl fmt::Display for LocalRegistry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.url, f)
    }
}
