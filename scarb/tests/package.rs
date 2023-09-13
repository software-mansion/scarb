use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::{Path, PathBuf};

use assert_fs::fixture::{ChildPath, PathChild};
use assert_fs::prelude::*;
use assert_fs::TempDir;
use indoc::{formatdoc, indoc};
use itertools::Itertools;

use scarb_test_support::command::Scarb;
use scarb_test_support::project_builder::ProjectBuilder;
use scarb_test_support::workspace_builder::WorkspaceBuilder;

struct PackageChecker {
    actual_files: HashMap<PathBuf, String>,
    base_name: PathBuf,
}

impl PackageChecker {
    fn open<'b>(path: &Path) -> tar::Archive<zstd::Decoder<'b, BufReader<File>>> {
        let path = ChildPath::new(path);
        path.assert(predicates::path::is_file());

        let file = File::open(&path).expect("failed to open package tarball");
        let reader = zstd::Decoder::new(file).expect("failed to create zstd decoder");
        tar::Archive::new(reader)
    }

    fn assert(path: &Path) -> Self {
        let mut archive = Self::open(path);

        let actual_files: HashMap<PathBuf, String> = archive
            .entries()
            .expect("failed to get archive entries")
            .map(|entry| {
                let mut entry = entry.expect("failed to get archive entry");
                let name = entry
                    .path()
                    .expect("failed to get archive entry path")
                    .into_owned();
                let mut contents = String::new();
                entry
                    .read_to_string(&mut contents)
                    .expect("failed to read archive entry contents");
                (name, contents)
            })
            .collect();

        let base_name = {
            let base_names = actual_files
                .keys()
                .map(|path| path.components().next().expect("empty path").as_os_str())
                .unique()
                .collect::<Vec<_>>();
            assert_eq!(
                base_names.len(),
                1,
                "multiple base names in package tarball: {}",
                base_names.iter().map(|p| p.to_string_lossy()).join(", ")
            );
            PathBuf::from(base_names.into_iter().next().unwrap())
        };

        Self {
            actual_files,
            base_name,
        }
    }

    fn name_and_version(&self, expected_package_name: &str, expected_version: &str) -> &Self {
        assert_eq!(
            self.base_name.to_string_lossy(),
            format!("{expected_package_name}-{expected_version}")
        );
        self
    }

    fn contents(&self, expected_files: &[&str]) -> &Self {
        let actual_files: HashSet<PathBuf> = self.actual_files.keys().cloned().collect();
        let expected_files: HashSet<PathBuf> = expected_files
            .iter()
            .map(|name| self.base_name.join(name))
            .collect();
        let missing: Vec<&PathBuf> = expected_files.difference(&actual_files).collect();
        let extra: Vec<&PathBuf> = actual_files.difference(&expected_files).collect();
        if !missing.is_empty() || !extra.is_empty() {
            panic!(
                "package tarball does not match.\nMissing: {:?}\nExtra: {:?}\n",
                missing, extra
            );
        }
        self
    }
}

#[test]
fn simple() {
    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("foo")
        .version("1.0.0")
        .lib_cairo("mod foo;")
        .src("src/foo.cairo", "fn foo() {}")
        .build(&t);

    Scarb::quick_snapbox()
        .arg("package")
        .current_dir(&t)
        .assert()
        .success()
        .stdout_matches(indoc! {r#"
        [..] Packaging foo v1.0.0 [..]
        [..]  Packaged 4 files, [..] ([..] compressed)
        "#});

    PackageChecker::assert(&t.child("target/package/foo-1.0.0.tar.zst"))
        .name_and_version("foo", "1.0.0")
        .contents(&["VERSION", "Scarb.orig.toml", "Scarb.toml", "src/lib.cairo"]);
}

#[test]
fn list_simple() {
    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("foo")
        .version("1.0.0")
        .build(&t);

    Scarb::quick_snapbox()
        .arg("package")
        .arg("--list")
        .current_dir(&t)
        .assert()
        .success()
        .stdout_eq(indoc! {r#"
            VERSION
            Scarb.orig.toml
            Scarb.toml
            src/lib.cairo
        "#});
}

#[test]
fn list_workspace() {
    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("first")
        .build(&t.child("first"));
    ProjectBuilder::start()
        .name("second")
        .build(&t.child("second"));
    WorkspaceBuilder::start()
        // Trick to test if packages are sorted alphabetically by name in the output.
        .add_member("second")
        .add_member("first")
        .build(&t);

    Scarb::quick_snapbox()
        .arg("package")
        .arg("--list")
        .current_dir(&t)
        .assert()
        .success()
        .stdout_eq(indoc! {r#"
            first:
            VERSION
            Scarb.orig.toml
            Scarb.toml
            src/lib.cairo

            second:
            VERSION
            Scarb.orig.toml
            Scarb.toml
            src/lib.cairo
        "#});
}

#[test]
fn reserved_files_collision() {
    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("foo")
        .version("1.0.0")
        .src("VERSION", "oops")
        .src("Scarb.orig.toml", "oops")
        .build(&t);

    Scarb::quick_snapbox()
        .arg("package")
        .current_dir(&t)
        .assert()
        .failure()
        .stdout_matches(formatdoc! {r#"
        [..] Packaging foo v1.0.0 [..]
        error: invalid inclusion of reserved files in package: VERSION, Scarb.orig.toml
        "#});
}

// TODO(mkaput): Manifest normalization, esp in workspaces
// TODO(mkaput): Git & local paths
// TODO(mkaput): Symlinks and other FS shenanigans
// TODO(mkaput): Gitignore
// TODO(mkaput): Invalid readme/license path
// TODO(mkaput): Restricted Windows files

#[test]
fn clean_tar_headers() {
    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("foo")
        .version("1.0.0")
        .lib_cairo("mod foo;")
        .src("src/foo.cairo", "fn foo() {}")
        .build(&t);

    Scarb::quick_snapbox()
        .arg("package")
        .current_dir(&t)
        .assert()
        .success();

    let mut archive = PackageChecker::open(&t.child("target/package/foo-1.0.0.tar.zst"));
    for entry in archive.entries().expect("failed to get archive entries") {
        let entry = entry.expect("failed to get archive entry");
        let header = entry.header();
        assert_eq!(header.mode().unwrap(), 0o644);
        assert_ne!(header.mtime().unwrap(), 0);
        assert_eq!(header.username().unwrap().unwrap(), "");
        assert_eq!(header.groupname().unwrap().unwrap(), "");
    }
}
