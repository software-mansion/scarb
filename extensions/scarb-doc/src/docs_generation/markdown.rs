use anyhow::Result;
use camino::Utf8Path;
use std::fs;

use crate::PackageInformation;
use crate::docs_generation::markdown::book_toml::generate_book_toml_content;
use crate::docs_generation::markdown::summary::generate_summary_file_content;
use crate::errors::{IODirectoryCreationError, IOWriteError};

mod book_toml;
mod context;
mod summary;
mod traits;

const BASE_HEADER_LEVEL: usize = 1;
const SOURCE_DIRECTORY: &str = "src";
const BOOK_TOML_FILENAME: &str = "book.toml";
pub const SUMMARY_FILENAME: &str = "SUMMARY.md";
const SHORT_DOCUMENTATION_LEN: usize = 200;

/// Prefixes that indicate the start of complex markdown structures,
/// such as tables. These should be avoided in brief documentation to maintain simple text
/// formatting and prevent disruption of the layout.
const SHORT_DOCUMENTATION_AVOID_PREFIXES: &[&str] = &["#", "\n\n", "```\n", "- ", "1.  "];

type Filename = String;
type GeneratedFile = (Filename, String);

pub struct MarkdownContent {
    book_toml: String,
    summary: String,
    doc_files: Vec<GeneratedFile>,
}

impl MarkdownContent {
    pub fn from_crate(package_information: &PackageInformation) -> Result<Self> {
        let (summary, doc_files) = generate_summary_file_content(&package_information.crate_)?;

        Ok(Self {
            book_toml: generate_book_toml_content(&package_information.metadata),
            summary,
            doc_files,
        })
    }

    pub fn save(self, output_dir: &Utf8Path) -> Result<()> {
        let source_directory_path = output_dir.join(SOURCE_DIRECTORY);
        fs::create_dir_all(&source_directory_path)
            .map_err(|e| IODirectoryCreationError::new(e, "generated documentation"))?;

        fs::write(output_dir.join(BOOK_TOML_FILENAME), self.book_toml)
            .map_err(|e| IOWriteError::new(e, "book.toml"))?;

        fs::write(source_directory_path.join(SUMMARY_FILENAME), self.summary)
            .map_err(|e| IOWriteError::new(e, "summary"))?;

        for (filename, file_content) in self.doc_files {
            fs::write(source_directory_path.join(filename.clone()), file_content)
                .map_err(|e| IOWriteError::new(e, filename.as_ref()))?;
        }

        Ok(())
    }
}
