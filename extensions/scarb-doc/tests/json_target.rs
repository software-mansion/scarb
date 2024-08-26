use std::{fs, path::Path};

use expect_test::{expect_file, ExpectFile};

#[derive(Default)]
pub struct JsonTargetChecker {
    actual: Option<String>,
    expected: Option<ExpectFile>,
}

impl JsonTargetChecker {
    pub fn actual(mut self, path: &Path) -> Self {
        self.actual = Option::Some(fs::read_to_string(path).expect("Failed to read actual file."));
        self
    }

    pub fn expected(mut self, path: &str) -> Self {
        self.expected = Option::Some(expect_file!(path));
        self
    }

    pub fn assert_files_match(self) {
        if self.actual.is_none() {
            panic!("error: actual target json file was not set.");
        }

        if self.expected.is_none() {
            panic!("error: expected target json file was not set.");
        }

        self.expected.unwrap().assert_eq(&self.actual.unwrap());
    }
}
