use crate::AdditionalMetadata;
use crate::doc_test::code_blocks::CodeBlock;
use anyhow::{Context, Result, anyhow};
use cairo_lang_filesystem::db::Edition;
use cairo_lang_parser::utils::SimpleParserDatabase;
use camino::{Utf8Path, Utf8PathBuf};
use indoc::formatdoc;
use scarb_build_metadata::CAIRO_VERSION;
use scarb_ui::Ui;
use std::fmt::Write;
use std::fs;
use tempfile::{TempDir, tempdir};

pub(crate) struct DocTestWorkspace {
    _temp_dir: TempDir,
    root: Utf8PathBuf,
    package_name: String,
    has_lib_target: bool,
}

impl DocTestWorkspace {
    pub fn new(
        metadata: &AdditionalMetadata,
        index: usize,
        code_block: &CodeBlock,
        has_lib_target: bool,
        ui: &Ui,
    ) -> Result<Self> {
        let temp_dir = tempdir().context("failed to create temporary workspace")?;
        let root = Utf8PathBuf::from_path_buf(temp_dir.path().to_path_buf())
            .map_err(|path| anyhow!("path `{}` is not UTF-8 encoded", path.display()))?;

        let package_name = format!("{}_example_{}", metadata.name, index);

        let workspace = Self {
            _temp_dir: temp_dir,
            root,
            package_name,
            has_lib_target,
        };
        workspace.write_manifest(metadata)?;
        workspace.write_src(&code_block.content, &metadata.name, ui)?;

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

    fn write_src(&self, content: &str, package_name: &str, ui: &Ui) -> Result<()> {
        let src_dir = self.root().join("src");
        fs::create_dir_all(&src_dir).context("failed to create src directory")?;

        let db = SimpleParserDatabase::default();
        let mut wrapped_body_candidate =
            String::with_capacity(content.len() + content.lines().count() * 5);
        writeln!(wrapped_body_candidate, "fn main() {{")?;
        for line in content.lines() {
            writeln!(wrapped_body_candidate, "    {}", line)?;
        }
        writeln!(wrapped_body_candidate, "}}")?;

        let is_function_body = db.parse_virtual(&wrapped_body_candidate).is_ok();

        let package_import = if is_function_body && !self.has_lib_target {
            ui.warn(formatdoc!(
                r#"
                package `{package_name}` has no `lib` target defined
                `{package_name}` contents cannot be imported in the doc string definition
            "#
            ));
            String::new()
        } else {
            format!("use {package_name}::*;")
        };

        let body = if is_function_body {
            wrapped_body_candidate
        } else {
            content.to_string()
        };
        let lib_cairo = formatdoc! {r#"
            {package_import}

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
