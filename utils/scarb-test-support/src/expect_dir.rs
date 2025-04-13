use crate::fsx;
use expect_test::expect_file;
use indoc::formatdoc;
use itertools::Itertools;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::{fs, iter::zip};
use walkdir::WalkDir;

pub struct ExpectDir {
    actual: Option<WalkDir>,
    actual_root: Option<PathBuf>,
    expected: Option<WalkDir>,
    expected_root: Option<PathBuf>,
    strict: bool,
}

impl Default for ExpectDir {
    fn default() -> Self {
        Self::strict()
    }
}

impl ExpectDir {
    /// Both actual and expected directories must be identical.
    pub fn strict() -> Self {
        Self {
            actual: Default::default(),
            expected: Default::default(),
            actual_root: Default::default(),
            expected_root: Default::default(),
            strict: true,
        }
    }

    /// The actual directory can have more entries, than the expected one.
    /// We only assert that the expected files are identical.
    pub fn lenient() -> Self {
        Self {
            actual: Default::default(),
            expected: Default::default(),
            actual_root: Default::default(),
            expected_root: Default::default(),

            strict: false,
        }
    }

    pub fn actual(mut self, path: &str) -> Self {
        assert!(Self::check_if_directory(path));
        self.actual_root = Some(fsx::canonicalize(path).unwrap());
        self.actual = Some(WalkDir::new(path).sort_by_file_name());
        self
    }

    pub fn expected(mut self, path: &str) -> Self {
        assert!(Self::check_if_directory(path));
        self.expected_root = Some(fsx::canonicalize(path).unwrap());
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

        let panic_on_counts = || {
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

        let actual_files = if self.strict {
            if actual_files.len() != expected_files.len() {
                panic_on_counts()
            };
            actual_files
        } else {
            let strip_path = |path: PathBuf, root: &Path| {
                let path = fsx::canonicalize(&path).unwrap();
                path.strip_prefix(root).unwrap().to_path_buf()
            };

            let actual_paths = expected_files
                .iter()
                .map(|expected| {
                    let expected = expected.path().to_path_buf();
                    strip_path(expected, self.expected_root.as_ref().unwrap())
                })
                .collect::<HashSet<_>>();

            let actual_files = actual_files
                .clone()
                .into_iter()
                .filter(|actual| {
                    let actual = actual.path().to_path_buf();
                    let actual = strip_path(actual, self.actual_root.as_ref().unwrap());
                    actual_paths.contains(&actual)
                })
                .collect_vec();

            if actual_files.len() != expected_files.len() {
                panic_on_counts()
            }

            actual_files
        };

        for (actual_dir_entry, expected_dir_entry) in zip(actual_files, expected_files) {
            if expected_dir_entry.file_type().is_file() {
                assert!(actual_dir_entry.file_type().is_file());

                let content = fs::read_to_string(actual_dir_entry.path())
                    .unwrap_or_else(|_| {
                        panic!("file read failed: {}", actual_dir_entry.path().display())
                    })
                    .replace("\r", "");

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
