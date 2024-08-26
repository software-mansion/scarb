use expect_test::expect_file;
use indoc::formatdoc;
use scarb_test_support::fsx;
use std::{fs, iter::zip};
use walkdir::WalkDir;

#[derive(Default)]
pub struct MarkdownTargetChecker {
    actual: Option<WalkDir>,
    expected: Option<WalkDir>,
}

impl MarkdownTargetChecker {
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

        let actual_files: Vec<_> = self
            .actual
            .unwrap()
            .into_iter()
            .filter_map(Result::ok)
            .collect();

        let expected_files: Vec<_> = self
            .expected
            .unwrap()
            .into_iter()
            .filter_map(Result::ok)
            .collect();

        if actual_files.len() != expected_files.len() {
            panic!(
                "{}",
                formatdoc! {
                    "
                    error: actual and expected target directories have different number of entries
                    actual: {actual}
                    expected: {expected}
                    ",
                    actual = actual_files.len(),
                    expected = expected_files.len()
                }
            );
        };

        for (actual_dir_entry, expected_dir_entry) in zip(actual_files, expected_files) {
            if expected_dir_entry.file_type().is_file() {
                assert!(actual_dir_entry.file_type().is_file());

                let content = fs::read_to_string(actual_dir_entry.path()).unwrap();

                let expect_file =
                    expect_file![fsx::canonicalize(expected_dir_entry.path()).unwrap()];
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
