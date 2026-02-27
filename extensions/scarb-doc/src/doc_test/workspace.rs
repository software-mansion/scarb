use crate::AdditionalMetadata;
use crate::doc_test::code_blocks::CodeBlock;
use anyhow::{Context, Result, anyhow};
use cairo_lang_filesystem::db::Edition;
use camino::{Utf8Path, Utf8PathBuf};
use indoc::formatdoc;
use scarb_build_metadata::CAIRO_VERSION;
use std::fmt::Write;
use std::fs;
use tempfile::{TempDir, tempdir};

pub(crate) struct TestWorkspace {
    _temp_dir: TempDir,
    root: Utf8PathBuf,
    package_name: String,
}

impl TestWorkspace {
    pub fn new(
        metadata: &AdditionalMetadata,
        index: usize,
        code_block: &CodeBlock,
    ) -> Result<Self> {
        let temp_dir = tempdir().context("failed to create temporary workspace")?;
        let root = Utf8PathBuf::from_path_buf(temp_dir.path().to_path_buf())
            .map_err(|path| anyhow!("path `{}` is not UTF-8 encoded", path.display()))?;

        let package_name = format!("{}_example_{}", metadata.name, index);

        let workspace = Self {
            _temp_dir: temp_dir,
            root,
            package_name,
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

    fn write_manifest(&self, metadata: &AdditionalMetadata) -> Result<()> {
        let package_dir = metadata
            .manifest_path
            .parent()
            .context("package manifest path has no parent directory")?;

        let dep = &metadata.name;
        let dep_path = format!("{}", package_dir);
        let name = &self.package_name;
        let edition = edition_variant(Edition::latest());

        let manifest = formatdoc! {r#"
            [package]
            name = "{name}"
            version = "0.1.0"
            edition = "{edition}"

            [dependencies]
            {dep} = {{ path = "{dep_path}" }}
            cairo_execute = "{CAIRO_VERSION}"

            [cairo]
            enable-gas = false

            [executable]
        "#
        };
        fs::write(self.manifest_path(), manifest).context("failed to write manifest")?;
        Ok(())
    }

    fn write_src(&self, content: &str, package_name: &str) -> Result<()> {
        let src_dir = self.root().join("src");
        fs::create_dir_all(&src_dir).context("failed to create src directory")?;

        // TODO: (#2889) Improve this logic to be more precise
        let has_main_fn = content.lines().any(|line| {
            line.trim_start().starts_with("fn main()")
                || line.trim_start().starts_with("pub fn main()")
        });

        let body = if has_main_fn {
            content.to_string()
        } else {
            let mut body = String::with_capacity(content.len() + content.lines().count() * 5);
            writeln!(body, "fn main() {{")?;
            for line in content.lines() {
                writeln!(body, "    {}", line)?;
            }
            writeln!(body, "}}")?;
            body
        };
        let lib_cairo = formatdoc! {r#"
            use {package_name}::*;

            #[executable]
            {body}
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
