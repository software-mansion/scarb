use crate::metadata::CompilationUnit;
use anyhow::{anyhow, Context, Result};
use camino::{Utf8Path, Utf8PathBuf};
use semver::Version;
use serde::Serialize;
use walkdir::WalkDir;

#[derive(Debug, PartialEq, Clone, Serialize)]
pub enum CrateLocation {
    /// Main crate in a package
    Lib,
    /// Crate in the `tests/` directory
    Tests,
}

#[derive(Debug, PartialEq)]
pub struct TestCompilationTarget {
    pub crate_root: Utf8PathBuf,
    pub crate_name: String,
    pub crate_version: Version,
    pub crate_location: CrateLocation,
    pub lib_content: String,
}

pub fn collect_test_compilation_targets(
    package_name: &str,
    package_version: Version,
    package_path: &Utf8Path,
    compilation_unit: &CompilationUnit,
) -> Result<Vec<TestCompilationTarget>> {
    let package_source_file_path = compilation_unit.main_package_source_file_path();
    let mut compilation_targets = vec![TestCompilationTarget {
        crate_root: compilation_unit.main_package_source_root(),
        crate_name: package_name.to_string(),
        crate_version: package_version.clone(),
        crate_location: CrateLocation::Lib,
        lib_content: std::fs::read_to_string(package_source_file_path)
            .with_context(|| format!("failed to read = {package_source_file_path}"))?,
    }];

    let tests_dir_path = package_path.join("tests");
    if tests_dir_path.exists() {
        compilation_targets.push(TestCompilationTarget {
            crate_name: "tests".to_string(),
            crate_version: package_version,
            crate_location: CrateLocation::Tests,
            lib_content: get_or_create_test_lib_content(tests_dir_path.as_path())?,
            crate_root: tests_dir_path,
        });
    }

    Ok(compilation_targets)
}

fn get_or_create_test_lib_content(tests_folder_path: &Utf8Path) -> Result<String> {
    let tests_lib_path = tests_folder_path.join("lib.cairo");
    if tests_lib_path
        .try_exists()
        .with_context(|| format!("Can't check the existence of file = {tests_lib_path}"))?
    {
        return std::fs::read_to_string(&tests_lib_path).with_context(|| {
            format!("Can't read the content of the file = {tests_lib_path} to string")
        });
    }

    let mut content = String::new();
    for entry in WalkDir::new(tests_folder_path)
        .max_depth(1)
        .sort_by_file_name()
    {
        let entry = entry
            .with_context(|| format!("Failed to read directory at path = {tests_folder_path}"))?;
        let path = Utf8Path::from_path(entry.path())
            .ok_or_else(|| anyhow!("Failed to convert path = {:?} to Utf8Path", entry.path()))?;

        if path.is_file() && path.extension().unwrap_or_default() == "cairo" {
            let mod_name = path
                .file_stem()
                .unwrap_or_else(|| panic!("Path to test = {path} should have .cairo extension"));

            content.push_str(&format!("mod {mod_name};\n"));
        }
    }
    Ok(content)
}
