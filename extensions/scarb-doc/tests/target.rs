use expect_test::expect_file;
use indoc::formatdoc;
use itertools::Itertools;
use scarb_test_support::fsx;
use std::{fs, iter::zip};
use walkdir::WalkDir;

#[derive(Default)]
pub struct TargetChecker {
    actual: Option<WalkDir>,
    expected: Option<WalkDir>,
}

impl TargetChecker {
    pub fn actual(mut self, path: &str) -> Self {
        assert!(Self::check_if_directory(path));
        self.actual = Some(WalkDir::new(path).sort_by_file_name());
        self
    }

    pub fn expected(mut self, path: &str) -> Self {
        assert!(Self::check_if_directory(path));
        self.expected = Some(WalkDir::new(path).sort_by_file_name());
        self
    }

    pub fn assert_all_files_match(self) {
        if self.actual.is_none() {
            panic!("error: actual target directory was not set.");
        }

        if self.expected.is_none() {
            panic!("error: expected target directory was not set.");
        }

        let actual_len = match self.actual.iter().try_len() {
            Ok(length) => length,
            Err(_) => self.actual.iter().count(),
        };

        let expected_len = match self.expected.iter().try_len() {
            Ok(length) => length,
            Err(_) => self.expected.iter().count(),
        };

        if actual_len != expected_len {
            panic!(
                "{}",
                formatdoc! {
                    "
                    error: actual and expected target directories have different number of entries
                    actual: {actual}
                    expected: {expected}
                    ",
                    actual = actual_len,
                    expected = expected_len
                }
            );
        };

        for (actual_dir_entry, expected_dir_entry) in
            zip(self.actual.unwrap(), self.expected.unwrap())
        {
            let expected_entry = expected_dir_entry.unwrap();
            let actual_entry = actual_dir_entry.unwrap();
            println!(
                "{} and {}",
                expected_entry.path().to_str().unwrap(),
                actual_entry.path().to_str().unwrap()
            );

            if expected_entry.file_type().is_file() {
                assert!(actual_entry.file_type().is_file());

                let content = fs::read_to_string(actual_entry.path()).unwrap();

                let expect_file = expect_file![fsx::canonicalize(expected_entry.path()).unwrap()];
                expect_file.assert_eq(&content);
            }
        }
    }

    fn check_if_directory(path: &str) -> bool {
        match fs::metadata(path) {
            Ok(metadata) => {
                if metadata.is_file() {
                    panic!(
                        "Given path ({wrong_path}) is not a directory.",
                        wrong_path = path
                    )
                }
            }
            Err(e) => panic!(
                "Failed to get metadata for {wrong_path}, {error}",
                wrong_path = path,
                error = e
            ),
        }
        true
    }
}
