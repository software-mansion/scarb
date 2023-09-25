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
use scarb_test_support::gitx;
use scarb_test_support::project_builder::{Dep, DepBuilder, ProjectBuilder};
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

    fn read_file(&self, path: impl AsRef<Path>) -> &str {
        let path = self.base_name.join(path);
        self.actual_files
            .get(&path)
            .unwrap_or_else(|| panic!("missing file in package tarball: {}", path.display()))
    }

    fn file_eq(&self, path: impl AsRef<Path>, expected_contents: &str) -> &Self {
        snapbox::assert_eq(expected_contents, self.read_file(path));
        self
    }

    fn file_eq_nl(&self, path: impl AsRef<Path>, expected_contents: &str) -> &Self {
        let mut expected_contents = expected_contents.to_owned();
        expected_contents.push('\n');
        self.file_eq(path, &expected_contents)
    }

    fn file_eq_path(&self, path: impl AsRef<Path>, expected_path: impl AsRef<Path>) -> &Self {
        snapbox::assert_eq_path(expected_path, self.read_file(path));
        self
    }

    fn file_matches(&self, path: impl AsRef<Path>, expected_contents: &str) -> &Self {
        snapbox::assert_matches(expected_contents, self.read_file(path));
        self
    }

    fn file_matches_nl(&self, path: impl AsRef<Path>, expected_contents: &str) -> &Self {
        let mut expected_contents = expected_contents.to_owned();
        expected_contents.push('\n');
        self.file_matches(path, &expected_contents)
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
        .contents(&["VERSION", "Scarb.orig.toml", "Scarb.toml", "src/lib.cairo"])
        .file_eq("VERSION", "1")
        .file_eq_path("Scarb.orig.toml", t.child("Scarb.toml"))
        .file_eq_path("src/lib.cairo", t.child("src/lib.cairo"))
        .file_eq_nl(
            "Scarb.toml",
            indoc! {r#"
                # Code generated by scarb package -p foo; DO NOT EDIT.
                #
                # When uploading packages to the registry Scarb will automatically
                # "normalize" Scarb.toml files for maximal compatibility
                # with all versions of Scarb and also rewrite `path` dependencies
                # to registry dependencies.
                #
                # If you are reading this file be aware that the original Scarb.toml
                # will likely look very different (and much more reasonable).
                # See Scarb.orig.toml for the original contents.

                [package]
                name = "foo"
                version = "1.0.0"

                [dependencies]
            "#},
        );
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

#[test]
fn generated_manifest() {
    let t = TempDir::new().unwrap();

    let path_dep = t.child("path_dep");
    ProjectBuilder::start()
        .name("path_dep")
        .version("0.1.0")
        .build(&path_dep);

    let git_dep = gitx::new("git_dep", |t| {
        ProjectBuilder::start()
            .name("git_dep")
            .version("0.2.0")
            .build(&t);
    });

    ProjectBuilder::start()
        .name("hello")
        .version("1.0.0")
        // TODO(mkaput): Uncomment this when registry source will be implemented.
        // .dep("registry_dep", Dep.version("1.0.0"))
        .dep("path_dep", path_dep.version("0.1.0"))
        .dep("git_dep", git_dep.version("0.2.0"))
        .dep_starknet()
        .manifest_extra(indoc! {r#"
            [tool.foobar]
            hello-world = { s = "s", n = 1 }

            [tool.fmt]
            sort-module-level-items = true
        "#})
        .build(&t);

    Scarb::quick_snapbox()
        .arg("package")
        .current_dir(&t)
        .assert()
        .success();

    PackageChecker::assert(&t.child("target/package/hello-1.0.0.tar.zst")).file_matches_nl(
        "Scarb.toml",
        indoc! {r#"
            # Code generated by scarb package -p hello; DO NOT EDIT.
            #
            # When uploading packages to the registry Scarb will automatically
            # "normalize" Scarb.toml files for maximal compatibility
            # with all versions of Scarb and also rewrite `path` dependencies
            # to registry dependencies.
            #
            # If you are reading this file be aware that the original Scarb.toml
            # will likely look very different (and much more reasonable).
            # See Scarb.orig.toml for the original contents.

            [package]
            name = "hello"
            version = "1.0.0"

            [dependencies.git_dep]
            version = "^0.2.0"

            [dependencies.path_dep]
            version = "^0.1.0"

            [dependencies.starknet]
            version = "[..]"

            [tool.fmt]
            sort-module-level-items = true

            [tool.foobar.hello-world]
            n = 1
            s = "s"
        "#},
    );
}

#[test]
fn workspace() {
    let t = TempDir::new().unwrap();
    let hello = t.child("hello");
    let path_dep = t.child("path_dep");
    let workspace_dep = t.child("workspace_dep");

    ProjectBuilder::start()
        .name("path_dep")
        .version("1.0.0")
        .build(&path_dep);

    ProjectBuilder::start()
        .name("workspace_dep")
        .version("1.0.0")
        .build(&workspace_dep);

    ProjectBuilder::start()
        .name("hello")
        .version("1.0.0")
        .dep("path_dep", Dep.workspace())
        .dep("workspace_dep", workspace_dep.version("1.0.0"))
        .manifest_extra(indoc! {r#"
            [tool]
            fmt.workspace = true
        "#})
        .build(&hello);

    WorkspaceBuilder::start()
        .add_member("hello")
        .add_member("workspace_dep")
        .dep("path_dep", path_dep.version("1.0.0"))
        .manifest_extra(indoc! {r#"
            [workspace.tool.fmt]
            sort-module-level-items = true
        "#})
        .build(&t);

    Scarb::quick_snapbox()
        .arg("package")
        .arg("--workspace")
        .current_dir(&t)
        .assert()
        .success()
        .stdout_matches(indoc! {r#"
        [..] Packaging hello v1.0.0 [..]
        [..]  Packaged [..]
        [..] Packaging workspace_dep v1.0.0 [..]
        [..]  Packaged [..]
        "#});

    PackageChecker::assert(&t.child("target/package/hello-1.0.0.tar.zst"))
        .name_and_version("hello", "1.0.0")
        .contents(&["VERSION", "Scarb.orig.toml", "Scarb.toml", "src/lib.cairo"])
        .file_eq("VERSION", "1")
        .file_eq_path("Scarb.orig.toml", hello.child("Scarb.toml"))
        .file_eq_path("src/lib.cairo", hello.child("src/lib.cairo"))
        .file_eq_nl(
            "Scarb.toml",
            indoc! {r#"
                # Code generated by scarb package -p hello; DO NOT EDIT.
                #
                # When uploading packages to the registry Scarb will automatically
                # "normalize" Scarb.toml files for maximal compatibility
                # with all versions of Scarb and also rewrite `path` dependencies
                # to registry dependencies.
                #
                # If you are reading this file be aware that the original Scarb.toml
                # will likely look very different (and much more reasonable).
                # See Scarb.orig.toml for the original contents.

                [package]
                name = "hello"
                version = "1.0.0"

                [dependencies.path_dep]
                version = "^1.0.0"

                [dependencies.workspace_dep]
                version = "^1.0.0"

                [tool.fmt]
                sort-module-level-items = true
            "#},
        );

    PackageChecker::assert(&t.child("target/package/workspace_dep-1.0.0.tar.zst"))
        .name_and_version("workspace_dep", "1.0.0");
}

#[test]
fn path_dependency_no_version() {
    let t = TempDir::new().unwrap();
    let hello = t.child("hello");
    let path_dep = t.child("path_dep");

    ProjectBuilder::start()
        .name("path_dep")
        .version("1.0.0")
        .build(&path_dep);

    ProjectBuilder::start()
        .name("hello")
        .version("1.0.0")
        .dep("path_dep", &path_dep)
        .build(&hello);

    Scarb::quick_snapbox()
        .arg("package")
        .current_dir(&hello)
        .assert()
        .failure()
        .stdout_matches(indoc! {r#"
        [..] Packaging hello v1.0.0 [..]
        error: dependency `path_dep` does not specify a version requirement
        note: all dependencies must have a version specified when packaging
        note: the `path` specification will be removed from dependency declaration
        "#});
}

#[test]
fn git_dependency_no_version() {
    let t = TempDir::new().unwrap();

    let git_dep = gitx::new("git_dep", |t| {
        ProjectBuilder::start()
            .name("git_dep")
            .version("1.0.0")
            .build(&t);
    });

    ProjectBuilder::start()
        .name("hello")
        .version("1.0.0")
        .dep("git_dep", &git_dep)
        .build(&t);

    Scarb::quick_snapbox()
        .arg("package")
        .current_dir(&t)
        .assert()
        .failure()
        .stdout_matches(indoc! {r#"
        [..] Updating git repository [..]
        [..] Packaging hello v1.0.0 [..]
        error: dependency `git_dep` does not specify a version requirement
        note: all dependencies must have a version specified when packaging
        note: the `git` specification will be removed from dependency declaration
        "#});
}

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
