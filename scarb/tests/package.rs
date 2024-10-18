#![allow(clippy::items_after_test_module)]

use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::{Path, PathBuf};

use assert_fs::fixture::{ChildPath, PathChild};
use assert_fs::prelude::*;
use assert_fs::TempDir;
use indoc::{formatdoc, indoc};
use itertools::Itertools;
use scarb::DEFAULT_TARGET_DIR_NAME;
use scarb_build_metadata::CAIRO_VERSION;
use scarb_test_support::cairo_plugin_project_builder::CairoPluginProjectBuilder;
use scarb_test_support::command::Scarb;
use scarb_test_support::fsx::unix_paths_to_os_lossy;
use scarb_test_support::gitx;
use scarb_test_support::project_builder::{Dep, DepBuilder, ProjectBuilder};
use scarb_test_support::registry::local::LocalRegistry;
use scarb_test_support::workspace_builder::WorkspaceBuilder;
use test_case::test_case;

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

fn simple_project() -> ProjectBuilder {
    ProjectBuilder::start()
        .name("foo")
        .version("1.0.0")
        .lib_cairo("mod foo;")
        .src("src/foo.cairo", "fn foo() {}")
        // Test that files we want not to be included are indeed not included.
        .lock("")
        .src("target/dev/evil.txt", "")
}

fn symlink_dir<P: AsRef<Path>, Q: AsRef<Path>>(original: P, link: Q) {
    #[cfg(unix)]
    use std::os::unix::fs::symlink as symlink_dir;
    #[cfg(windows)]
    use std::os::windows::fs::symlink_dir;

    let original = original.as_ref();
    let link = link.as_ref();
    symlink_dir(original, link).unwrap_or_else(|e| {
        panic!(
            "failed to create symlink from {} to {}: {}",
            original.display(),
            link.display(),
            e
        )
    });
}

#[test]
fn simple() {
    let t = TempDir::new().unwrap();
    simple_project().build(&t);

    Scarb::quick_snapbox()
        .arg("package")
        .arg("--no-metadata")
        .current_dir(&t)
        .assert()
        .success()
        .stdout_matches(indoc! {r#"
        [..] Packaging foo v1.0.0 [..]
        [..] Verifying foo-1.0.0.tar.zst
        [..] Compiling foo v1.0.0 ([..])
        [..]  Finished `dev` profile target(s) in [..]
        [..]  Packaged [..] files, [..] ([..] compressed)
        "#});

    PackageChecker::assert(&t.child("target/package/foo-1.0.0.tar.zst"))
        .name_and_version("foo", "1.0.0")
        .contents(&[
            "VERSION",
            "Scarb.orig.toml",
            "Scarb.toml",
            "src/lib.cairo",
            "src/foo.cairo",
        ])
        .file_eq("VERSION", "1")
        .file_eq_path("Scarb.orig.toml", t.child("Scarb.toml"))
        .file_eq_path("src/lib.cairo", t.child("src/lib.cairo"))
        .file_eq_path("src/foo.cairo", t.child("src/foo.cairo"))
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
                edition = "2023_01"

                [dependencies]
            "#},
        );
}

#[test]
fn list_simple() {
    let t = TempDir::new().unwrap();
    simple_project().build(&t);

    Scarb::quick_snapbox()
        .arg("package")
        .arg("--list")
        .current_dir(&t)
        .assert()
        .success()
        .stdout_eq(unix_paths_to_os_lossy(indoc! {r#"
            VERSION
            Scarb.orig.toml
            Scarb.toml
            src/foo.cairo
            src/lib.cairo
        "#}));
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
        .stdout_eq(unix_paths_to_os_lossy(indoc! {r#"
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
        "#}));
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
        .arg("--no-metadata")
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

    let mut registry = LocalRegistry::create();
    registry.publish(|t| {
        ProjectBuilder::start()
            .name("registry_dep")
            .version("1.0.0")
            .build(t);
    });

    ProjectBuilder::start()
        .name("hello")
        .version("1.0.0")
        .dep("registry_dep", Dep.version("1.0.0").registry(&registry))
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
        .arg("--no-verify")
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
            edition = "2023_01"

            [dependencies.git_dep]
            version = "^0.2.0"

            [dependencies.path_dep]
            version = "^0.1.0"

            [dependencies.registry_dep]
            version = "^1.0.0"
            registry = "file://[..]"

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
        .arg("--no-verify")
        .arg("--no-metadata")
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
                edition = "2023_01"

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
fn cairo_plugin() {
    let t = TempDir::new().unwrap();
    CairoPluginProjectBuilder::default().build(&t);

    Scarb::quick_snapbox()
        .arg("package")
        .arg("--no-metadata")
        // Disable output from Cargo.
        .env("CARGO_TERM_QUIET", "true")
        .current_dir(&t)
        .assert()
        .success()
        .stdout_matches(indoc! {r#"
        [..] Packaging some v1.0.0 [..]
        [..]warn: package name or version differs between Cargo manifest and Scarb manifest
        [..]Scarb manifest: `some-1.0.0`, Cargo manifest: `some-0.1.0`
        [..]this might become an error in future Scarb releases

        [..] Verifying some-1.0.0.tar.zst
        [..] Compiling some v1.0.0 ([..])
        [..]  Finished `dev` profile target(s) in [..]
        [..]  Packaged [..] files, [..] ([..] compressed)
        "#});

    PackageChecker::assert(&t.child("target/package/some-1.0.0.tar.zst"))
        .name_and_version("some", "1.0.0")
        .contents(&[
            "VERSION",
            "Scarb.orig.toml",
            "Scarb.toml",
            "Cargo.orig.toml",
            "Cargo.toml",
            "src/lib.rs",
        ])
        .file_eq("VERSION", "1")
        .file_eq_path("src/lib.rs", t.child("src/lib.rs"))
        .file_eq_path("Scarb.orig.toml", t.child("Scarb.toml"))
        .file_eq_nl(
            "Scarb.toml",
            indoc! {r#"
                # Code generated by scarb package -p some; DO NOT EDIT.
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
                name = "some"
                version = "1.0.0"
                edition = "2023_01"

                [dependencies]

                [cairo-plugin]
                name = "some"
            "#},
        )
        .file_matches_nl(
            "Cargo.orig.toml",
            indoc! {r#"
                [package]
                name = "some"
                version = "0.1.0"
                edition = "2021"
                publish = false

                [lib]
                crate-type = ["cdylib"]

                [dependencies]
                cairo-lang-macro = { path = "[..]cairo-lang-macro", version = "0.1.0" }
            "#},
        )
        .file_matches(
            "Cargo.toml",
            indoc! {r#"
                # THIS FILE IS AUTOMATICALLY GENERATED BY CARGO
                #
                ...
            "#},
        );
}

#[test]
fn builtin_cairo_plugin() {
    let t = TempDir::new().unwrap();
    CairoPluginProjectBuilder::start()
        .name("assert_macros")
        .scarb_project(|b| {
            b.name("assert_macros")
                .version(CAIRO_VERSION)
                .manifest_package_extra("no-core = true")
                .manifest_extra(indoc! {r#"
                    [cairo-plugin]
                    builtin = true
                "#})
        })
        .build(&t);

    Scarb::quick_snapbox()
        .arg("package")
        .arg("--no-metadata")
        .current_dir(&t)
        .assert()
        .success()
        .stdout_matches(formatdoc! {r#"
            [..]Packaging assert_macros v{CAIRO_VERSION} ([..]Scarb.toml)
            [..]Packaged [..] files, [..] ([..] compressed)
        "#});

    PackageChecker::assert(&t.child(format!(
        "target/package/assert_macros-{CAIRO_VERSION}.tar.zst"
    )))
    .name_and_version("assert_macros", CAIRO_VERSION)
    .contents(&["VERSION", "Scarb.orig.toml", "Scarb.toml"])
    .file_eq("VERSION", "1")
    .file_eq_path("Scarb.orig.toml", t.child("Scarb.toml"))
    .file_eq_nl(
        "Scarb.toml",
        formatdoc! {r#"
                # Code generated by scarb package -p assert_macros; DO NOT EDIT.
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
                name = "assert_macros"
                version = "{CAIRO_VERSION}"
                edition = "2023_01"
                no-core = true

                [dependencies]

                [cairo-plugin]
                name = "assert_macros"
                builtin = true
            "#}
        .as_str(),
    );
}

#[test]
fn clean_repo() {
    let t = TempDir::new().unwrap();

    simple_project().build(&t);
    t.child(".gitignore")
        .write_str(DEFAULT_TARGET_DIR_NAME)
        .unwrap();
    gitx::init(&t);

    // Fetch is run to make sure that Scarb.lock is created before the repo init.
    // Otherwise random changes preventing packaging the project might occur.
    Scarb::quick_snapbox()
        .current_dir(&t)
        .arg("fetch")
        .assert()
        .success();

    t.child("src/bar.cairo").write_str("fn bar() {}").unwrap();
    gitx::commit(&t);

    Scarb::quick_snapbox()
        .current_dir(&t)
        .arg("package")
        .assert()
        .success();

    PackageChecker::assert(&t.child("target/package/foo-1.0.0.tar.zst"))
        .name_and_version("foo", "1.0.0")
        .contents(&[
            "VERSION",
            "VCS.json",
            "Scarb.orig.toml",
            "Scarb.toml",
            "src/lib.cairo",
            "src/foo.cairo",
            "src/bar.cairo",
        ])
        .file_matches("VCS.json", r#"{"git":{"sha1":"[..]"},"path_in_vcs":""}"#);
}

#[test]
fn dirty_repo() {
    let t = TempDir::new().unwrap();

    simple_project().build(&t);
    gitx::init(&t);
    gitx::commit(&t);

    t.child("src/bar.cairo").write_str("fn bar() {}").unwrap();

    Scarb::quick_snapbox()
        .arg("package")
        .arg("--no-metadata")
        .current_dir(&t)
        .assert()
        .failure()
        .stdout_matches(indoc! {r#"
            [..] Packaging foo v1.0.0 [..]
            error: cannot package a repository containing uncommitted changes
            help: to proceed despite this and include the uncommitted changes, pass the `--allow-dirty` flag
        "#});
}

#[test]
fn dirty_repo_allow_dirty() {
    let t = TempDir::new().unwrap();

    simple_project().build(&t);
    gitx::init(&t);
    gitx::commit(&t);

    t.child("src/bar.cairo").write_str("fn bar() {}").unwrap();

    Scarb::quick_snapbox()
        .arg("package")
        .arg("--allow-dirty")
        .current_dir(&t)
        .assert()
        .success();

    PackageChecker::assert(&t.child("target/package/foo-1.0.0.tar.zst"))
        .name_and_version("foo", "1.0.0")
        .contents(&[
            "VERSION",
            "VCS.json",
            "Scarb.orig.toml",
            "Scarb.toml",
            "src/lib.cairo",
            "src/foo.cairo",
            "src/bar.cairo",
        ])
        .file_matches("VCS.json", r#"{"git":{"sha1":"[..]"},"path_in_vcs":""}"#);
}

#[test]
fn repo_without_commits() {
    let t = TempDir::new().unwrap();

    simple_project().build(&t);
    gitx::init(&t);

    t.child("src/bar.cairo").write_str("fn bar() {}").unwrap();

    Scarb::quick_snapbox()
        .arg("package")
        .arg("--allow-dirty")
        .current_dir(&t)
        .assert()
        .success();

    PackageChecker::assert(&t.child("target/package/foo-1.0.0.tar.zst"))
        .name_and_version("foo", "1.0.0")
        .contents(&[
            "VERSION",
            "Scarb.orig.toml",
            "Scarb.toml",
            "src/lib.cairo",
            "src/foo.cairo",
            "src/bar.cairo",
        ]);
}

#[test]
fn list_clean_repo() {
    let t = TempDir::new().unwrap();

    simple_project().build(&t);
    gitx::init(&t);
    gitx::commit(&t);

    Scarb::quick_snapbox()
        .arg("package")
        .arg("--list")
        .current_dir(&t)
        .assert()
        .success()
        .stdout_eq(unix_paths_to_os_lossy(indoc! {r#"
            VERSION
            Scarb.orig.toml
            Scarb.toml
            VCS.json
            src/foo.cairo
            src/lib.cairo
        "#}));
}

#[test]
fn list_dirty_repo() {
    let t = TempDir::new().unwrap();

    simple_project().build(&t);
    gitx::init(&t);
    gitx::commit(&t);
    t.child("src/bar.cairo").write_str("fn bar() {}").unwrap();

    Scarb::quick_snapbox()
        .arg("package")
        .arg("--list")
        .current_dir(&t)
        .assert()
        .success()
        .stdout_eq(unix_paths_to_os_lossy(indoc! {r#"
            VERSION
            Scarb.orig.toml
            Scarb.toml
            VCS.json
            src/bar.cairo
            src/foo.cairo
            src/lib.cairo
        "#}));
}

#[test]
fn nested_package_vcs_path() {
    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("bar")
        .build(&t.child("foo/bar"));
    WorkspaceBuilder::start()
        // Trick to test if packages are sorted alphabetically by name in the output.
        .add_member("foo/bar")
        .build(&t);
    t.child(".gitignore")
        .write_str(DEFAULT_TARGET_DIR_NAME)
        .unwrap();

    gitx::init(&t);

    // Fetch is run to make sure that Scarb.lock is created before the repo init.
    // Otherwise random changes preventing packaging the project might occur.
    Scarb::quick_snapbox()
        .current_dir(&t)
        .arg("fetch")
        .assert()
        .success();

    gitx::commit(&t);

    Scarb::quick_snapbox()
        .current_dir(&t)
        .arg("package")
        .arg("-p")
        .arg("bar")
        .assert()
        .success();

    PackageChecker::assert(&t.child("target/package/bar-1.0.0.tar.zst"))
        .name_and_version("bar", "1.0.0")
        .contents(&[
            "VERSION",
            "VCS.json",
            "Scarb.orig.toml",
            "Scarb.toml",
            "src/lib.cairo",
        ])
        .file_matches(
            "VCS.json",
            r#"{"git":{"sha1":"[..]"},"path_in_vcs":"foo/bar"}"#,
        );
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
        .arg("--no-metadata")
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
        .arg("--no-metadata")
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

#[test]
fn list_ignore_nested() {
    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("foo")
        .version("1.0.0")
        .build(&t);
    ProjectBuilder::start()
        .name("child")
        .version("1.0.0")
        .build(&t.child("child"));

    Scarb::quick_snapbox()
        .arg("package")
        .arg("--list")
        .current_dir(&t)
        .assert()
        .success()
        .stdout_eq(unix_paths_to_os_lossy(indoc! {r#"
            VERSION
            Scarb.orig.toml
            Scarb.toml
            src/lib.cairo
        "#}));
}

#[test]
fn include_readme_and_license() {
    let t = TempDir::new().unwrap();

    t.child("Scarb.toml")
        .write_str(indoc! { r#"
            [package]
            name = "foo"
            version = "1.0.0"
            license-file = "LICENSE.txt"
        "# })
        .unwrap();
    t.child("src/lib.cairo").write_str("fn foo() {}").unwrap();
    t.child("README").write_str("README file").unwrap();
    t.child("LICENSE.txt")
        .write_str("This is LICENSE file")
        .unwrap();

    Scarb::quick_snapbox()
        .current_dir(&t)
        .arg("package")
        .arg("--allow-dirty")
        .assert()
        .success();

    PackageChecker::assert(&t.child("target/package/foo-1.0.0.tar.zst"))
        .name_and_version("foo", "1.0.0")
        .contents(&[
            "LICENSE",
            "README.md",
            "VERSION",
            "Scarb.orig.toml",
            "Scarb.toml",
            "src/lib.cairo",
        ])
        .file_matches("LICENSE", "This is LICENSE file")
        .file_matches("README.md", "README file");
}

#[test]
fn include_readme_and_license_from_outside() {
    let t = TempDir::new().unwrap();

    t.child("README").write_str("This is README file").unwrap();
    t.child("LICENSE.txt")
        .write_str("This is LICENSE file")
        .unwrap();

    t.child("foo/Scarb.toml")
        .write_str(indoc! { r#"
            [package]
            name = "foo"
            version = "1.0.0"
            license-file = "../LICENSE.txt"
            readme = "../README"
        "# })
        .unwrap();
    t.child("foo/src/lib.cairo")
        .write_str("fn foo() {}")
        .unwrap();

    Scarb::quick_snapbox()
        .current_dir(t.child("foo"))
        .arg("package")
        .arg("--allow-dirty")
        .assert()
        .success();

    PackageChecker::assert(&t.child("foo/target/package/foo-1.0.0.tar.zst"))
        .name_and_version("foo", "1.0.0")
        .contents(&[
            "LICENSE",
            "README.md",
            "VERSION",
            "Scarb.orig.toml",
            "Scarb.toml",
            "src/lib.cairo",
        ])
        .file_matches("LICENSE", "This is LICENSE file")
        .file_matches("README.md", "This is README file");
}

#[test]
fn include_readme_and_license_from_workspace() {
    let t = TempDir::new().unwrap();

    t.child("LICENSE.md")
        .write_str("This is LICENSE file")
        .unwrap();
    t.child("MY_README")
        .write_str("This is README file")
        .unwrap();

    t.child("foo/Scarb.toml")
        .write_str(indoc! { r#"
            [package]
            name = "foo"
            version = "1.0.0"
            license-file.workspace = true
            readme.workspace = true
        "# })
        .unwrap();
    t.child("foo/src/lib.cairo")
        .write_str("fn foo() {}")
        .unwrap();

    WorkspaceBuilder::start()
        .manifest_extra(indoc! {r#"
            [workspace.package]
            license-file = "LICENSE.md"
            readme = "MY_README"
        "#})
        .add_member("foo")
        .build(&t);

    Scarb::quick_snapbox()
        .current_dir(&t)
        .arg("package")
        .arg("-p")
        .arg("foo")
        .arg("--allow-dirty")
        .assert()
        .success();

    PackageChecker::assert(&t.child("target/package/foo-1.0.0.tar.zst"))
        .name_and_version("foo", "1.0.0")
        .contents(&[
            "LICENSE",
            "README.md",
            "VERSION",
            "Scarb.orig.toml",
            "Scarb.toml",
            "src/lib.cairo",
        ])
        .file_matches("LICENSE", "This is LICENSE file")
        .file_matches("README.md", "This is README file");
}

#[test]
#[cfg_attr(
    target_family = "windows",
    ignore = "Windows doesn't allow these characters in filenames."
)]
fn weird_characters_in_filenames() {
    let t = TempDir::new().unwrap();
    ProjectBuilder::start().src("src/:foo", "").build(&t);

    Scarb::quick_snapbox()
        .arg("package")
        .arg("--no-metadata")
        .current_dir(&t)
        .assert()
        .failure()
        .stdout_matches(indoc! {r#"
        [..] Packaging [..]
        error: cannot package a filename with a special character `:`: src/:foo
        "#});
}

#[test]
#[cfg_attr(
    target_family = "windows",
    ignore = "We do not want to create invalid files on Windows."
)]
fn windows_restricted_filenames() {
    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .lib_cairo("mod aux;")
        .src("src/aux.cairo", "")
        .build(&t);

    Scarb::quick_snapbox()
        .arg("package")
        .arg("--no-metadata")
        .current_dir(&t)
        .assert()
        .failure()
        .stdout_matches(indoc! {r#"
        [..] Packaging [..]
        error: cannot package file `src/aux.cairo`, it is a Windows reserved filename
        "#});
}

/// This test requires you to be able to make symlinks.
/// For windows, this may require you to enable developer mode.
#[test]
fn package_symlink() {
    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("foo")
        .version("1.0.0")
        .build(&t);

    symlink_dir(t.child("src"), t.child("dup"));

    Scarb::quick_snapbox()
        .arg("package")
        .current_dir(&t)
        .assert()
        .success();

    PackageChecker::assert(&t.child("target/package/foo-1.0.0.tar.zst"))
        .name_and_version("foo", "1.0.0")
        .contents(&[
            "VERSION",
            "Scarb.orig.toml",
            "Scarb.toml",
            "src/lib.cairo",
            "dup/lib.cairo",
        ])
        .file_eq_path("src/lib.cairo", t.child("src/lib.cairo"))
        .file_eq_path("dup/lib.cairo", t.child("src/lib.cairo"));
}

#[test]
fn broken_symlink() {
    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("foo")
        .version("1.0.0")
        .build(&t);

    symlink_dir("nowhere", t.child("src/foo.cairo"));

    Scarb::quick_snapbox()
        .arg("package")
        .arg("--no-metadata")
        .current_dir(&t)
        .assert()
        .failure()
        .stdout_matches(indoc! {r#"
        [..] Packaging [..]
        error: failed to list source files in: [..]

        Caused by:
            [..]
        "#});
}

#[test]
fn broken_but_excluded_symlink() {
    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("foo")
        .version("1.0.0")
        .build(&t);

    symlink_dir("nowhere", t.child("target"));

    // FIXME(mkaput): Technically, we can just ignore such symlinks.
    Scarb::quick_snapbox()
        .arg("package")
        .arg("--no-metadata")
        .current_dir(&t)
        .assert()
        .failure()
        .stdout_matches(indoc! {r#"
        [..] Packaging [..]
        error: failed to list source files in: [..]

        Caused by:
            [..]
        "#});
}

#[test]
fn filesystem_loop() {
    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("foo")
        .version("1.0.0")
        .build(&t);

    symlink_dir(t.child("src/symlink/foo/bar/baz"), t.child("src/symlink"));

    Scarb::quick_snapbox()
        .arg("package")
        .arg("--no-metadata")
        .current_dir(&t)
        .assert()
        .failure()
        .stdout_matches(indoc! {r#"
        [..] Packaging [..]
        error: failed to list source files in: [..]

        Caused by:
            [..]
        "#});
}

#[test]
fn exclude_dot_files_and_directories_by_default() {
    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("foo")
        .version("1.0.0")
        .src(".dotfile", "")
        .src(".dotdir/file", "")
        .build(&t);

    Scarb::quick_snapbox()
        .arg("package")
        .arg("--list")
        .current_dir(&t)
        .assert()
        .success()
        .stdout_eq(unix_paths_to_os_lossy(indoc! {r#"
            VERSION
            Scarb.orig.toml
            Scarb.toml
            src/lib.cairo
        "#}));
}

#[test]
fn clean_tar_headers() {
    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("foo")
        .version("1.0.0")
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

#[test_case("../.gitignore", false, false; "gitignore outside")]
#[test_case("../.gitignore", true, false; "gitignore outside with git")]
#[test_case("../.ignore", false, false; "ignore outside")]
#[test_case("../.scarbignore", false, false; "scarbignore outside")]
#[test_case(".gitignore", false, false; "gitignore inside")]
#[test_case(".gitignore", true, true; "gitignore inside with git")]
#[test_case(".ignore", false, true; "ignore inside")]
#[test_case(".scarbignore", false, true; "scarbignore inside")]
fn ignore_file(ignore_path: &str, setup_git: bool, expect_ignore_to_work: bool) {
    let g = gitx::new_conditional(setup_git, "package", |t| {
        ProjectBuilder::start()
            .name("foo")
            .version("1.0.0")
            .src("ignore.txt", "")
            .src("noignore.txt", "")
            .build(&t);

        t.child(ignore_path)
            .write_str(indoc! {r#"
                *.txt
                !noignore.txt
            "#})
            .unwrap();
    });

    let mut expected = Vec::new();
    expected.push("VERSION");
    expected.push("Scarb.orig.toml");
    expected.push("Scarb.toml");
    if setup_git {
        expected.push("VCS.json");
    }
    if !expect_ignore_to_work {
        expected.push("ignore.txt");
    }
    expected.push("noignore.txt");
    expected.push("src/lib.cairo");
    expected.push(""); // Ensure there's trailing \n

    let expected = unix_paths_to_os_lossy(&expected.join("\n"));

    Scarb::quick_snapbox()
        .arg("package")
        .arg("--list")
        .current_dir(g.p)
        .assert()
        .success()
        .stdout_eq(expected);
}

#[test]
fn ignore_whitelist_pattern() {
    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("foo")
        .version("1.0.0")
        .src("ignore.txt", "")
        .src("noignore.txt", "")
        .src("src/ignore.txt", "")
        .build(&t);

    t.child(".scarbignore")
        .write_str(indoc! {r#"
            *
            !*/
            !Scarb.toml
            !src/
            !src/*
            src/ignore.*
            !noignore.txt
        "#})
        .unwrap();

    Scarb::quick_snapbox()
        .arg("package")
        .arg("--list")
        .current_dir(&t)
        .assert()
        .success()
        .stdout_eq(unix_paths_to_os_lossy(indoc! {r#"
            VERSION
            Scarb.orig.toml
            Scarb.toml
            noignore.txt
            src/lib.cairo
        "#}));
}

#[test]
fn no_target() {
    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("foo")
        .version("1.0.0")
        .manifest_extra(indoc! {r#"
        [[target.starknet-contract]]
        "#})
        .build(&t);

    Scarb::quick_snapbox()
        .arg("package")
        .arg("--no-metadata")
        .current_dir(&t)
        .assert()
        .failure()
        .stdout_matches(indoc! {r#"
        [..] Packaging foo v1.0.0 [..]
        error: cannot archive package `foo` without a `lib` or `cairo-plugin` target
        help: consider adding `[lib]` section to package manifest
         --> Scarb.toml
        +   [lib]
        "#});
}

#[test]
fn error_on_verification() {
    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("foo")
        .version("1.0.0")
        .src("src/lib.cairo", ".")
        .build(&t);

    Scarb::quick_snapbox()
        .arg("package")
        .arg("--no-metadata")
        .current_dir(&t)
        .assert()
        .failure()
        .stdout_matches(indoc! {r#"
        [..] Packaging foo v1.0.0 [..]
        [..] Verifying foo-1.0.0.tar.zst
        [..] Compiling foo v1.0.0 ([..])
        error: Skipped tokens. Expected: [..]
         --> [..]
        .
        ^

        error: failed to verify package tarball

        Caused by:
        [..] could not compile `foo` due to previous error
        "#});
}

#[test]
fn package_without_verification() {
    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("foo")
        .version("1.0.0")
        .src("src/lib.cairo", "fn foo().")
        .build(&t);

    Scarb::quick_snapbox()
        .arg("package")
        .arg("--no-verify")
        .arg("--no-metadata")
        .current_dir(&t)
        .assert()
        .success()
        .stdout_matches(indoc! {r#"
        [..] Packaging foo v1.0.0 [..]
        [..]  Packaged [..]
        "#});
}

#[test]
fn package_cairo_plugin_without_verification() {
    let t = TempDir::new().unwrap();
    CairoPluginProjectBuilder::default().build(&t);

    Scarb::quick_snapbox()
        .arg("package")
        .arg("--no-verify")
        .arg("--no-metadata")
        .env("CARGO_TERM_QUIET", "true")
        .current_dir(&t)
        .assert()
        .success()
        .stdout_matches(indoc! {r#"
        [..] Packaging some v1.0.0 [..]
        [..]warn: package name or version differs between Cargo manifest and Scarb manifest
        [..]Scarb manifest: `some-1.0.0`, Cargo manifest: `some-0.1.0`
        [..]this might become an error in future Scarb releases

        [..]  Packaged [..]
        "#});
}

#[test]
fn package_without_publish_metadata() {
    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("foo")
        .version("1.0.0")
        .build(&t);

    Scarb::quick_snapbox()
        .arg("package")
        .current_dir(&t)
        .assert()
        .success()
        .stdout_matches(indoc! {r#"
        [..] Packaging foo v1.0.0 [..]
        warn: manifest has no readme
        warn: manifest has no description
        warn: manifest has no license or license-file
        warn: manifest has no documentation or homepage or repository
        see https://docs.swmansion.com/scarb/docs/reference/manifest.html#package for more info

        [..] Verifying foo-1.0.0.tar.zst
        [..] Compiling foo v1.0.0 ([..])
        [..]  Finished `dev` profile target(s) in [..]
        [..]  Packaged [..] files, [..] ([..] compressed)
        "#});
}

#[test]
fn package_with_publish_disabled() {
    let t = TempDir::new().unwrap();
    ProjectBuilder::start()
        .name("foo")
        .version("1.0.0")
        .manifest_package_extra("publish = false")
        .build(&t);

    Scarb::quick_snapbox()
        .arg("package")
        .arg("--no-metadata")
        .arg("--no-verify")
        .current_dir(&t)
        .assert()
        .success()
        .stdout_matches(indoc! {r#"
            [..]Packaging foo v1.0.0 ([..]Scarb.toml)
            [..]Packaged [..] files, [..] ([..] compressed)
        "#});
}
