use crate::doc_test::code_blocks::CodeBlock;
use anyhow::{Context, Result, anyhow};
use cairo_lang_filesystem::db::Edition;
use camino::{Utf8Path, Utf8PathBuf};
use indoc::formatdoc;
use scarb_build_metadata::CAIRO_VERSION;
use scarb_metadata::PackageMetadata;
use std::fmt::Write;
use std::fs;
use tempfile::{TempDir, tempdir};

pub struct TestWorkspace {
    _temp_dir: TempDir,
    root: Utf8PathBuf,
    package_name: String,
    item_full_path: String,
}

impl TestWorkspace {
    pub fn new(metadata: &PackageMetadata, index: usize, code_block: &CodeBlock) -> Result<Self> {
        let temp_dir = tempdir().context("failed to create temporary workspace")?;
        let root = Utf8PathBuf::from_path_buf(temp_dir.path().to_path_buf())
            .map_err(|path| anyhow!("path `{}` is not UTF-8 encoded", path.display()))?;

        let package_name = format!("{}_example_{}", metadata.name, index);

        let workspace = Self {
            _temp_dir: temp_dir,
            root,
            package_name,
            item_full_path: code_block.id.item_full_path.clone(),
        };
        workspace.write_manifest(metadata)?;
        workspace.write_src(&code_block.content, &metadata.name)?;

        Ok(workspace)
    }

    pub fn root(&self) -> &Utf8Path {
        &self.root
    }

    pub fn manifest_path(&self) -> Utf8PathBuf {
        self.root.join("Scarb.toml")
    }

    pub fn package_name(&self) -> &str {
        &self.package_name
    }

    pub fn item_full_path(&self) -> &str {
        &self.item_full_path
    }

    fn write_manifest(&self, metadata: &PackageMetadata) -> Result<()> {
        let package_dir = metadata
            .manifest_path
            .parent()
            .context("package manifest path has no parent directory")?;

        let dep = &metadata.name;
        let dep_path = format!("\"{}\"", package_dir);
        let name = &self.package_name;
        let edition = edition_variant(Edition::latest());

        let manifest = formatdoc! {r#"
            [package]
            name = "{name}"
            version = "0.1.0"
            edition = "{edition}"

            [dependencies]
            {dep} = {{ path = {dep_path} }}
            cairo_execute = "{CAIRO_VERSION}"

            [cairo]
            enable-gas = false

            [executable]
        "#
        };
        fs::write(&self.manifest_path(), manifest).context("failed to write manifest")?;
        Ok(())
    }

    fn write_src(&self, content: &str, package_name: &str) -> Result<()> {
        let src_dir = self.root().join("src");
        fs::create_dir_all(&src_dir).context("failed to create src directory")?;

        let mut body = String::with_capacity(content.len() + content.lines().count() * 5);
        for line in content.lines() {
            writeln!(body, "    {}", line)?;
        }
        let lib_cairo = formatdoc! {r#"
            use {package_name}::*;

            #[executable]
            fn main() {{
            {body}
            }}
        "#};
        fs::write(src_dir.join("lib.cairo"), lib_cairo).context("failed to write lib.cairo")?;
        Ok(())
    }
}

fn edition_variant(edition: Edition) -> String {
    let edition = serde_json::to_value(edition).unwrap();
    let serde_json::Value::String(edition) = edition else {
        panic!("Edition should always be a string.")
    };
    edition
}
