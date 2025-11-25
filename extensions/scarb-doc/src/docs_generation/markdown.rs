use crate::PackageInformation;
use crate::docs_generation::markdown::book_toml::generate_book_toml_content;
use crate::docs_generation::markdown::summary::generate_summary_file_content;
use crate::errors::{IODirectoryCreationError, IOWriteError};
use anyhow::Result;
use camino::Utf8Path;
use std::collections::HashMap;
use std::fs;

mod book_toml;
pub mod context;
mod summary;
pub mod traits;
use crate::docs_generation::common::{
    GeneratedFile, OutputFilesExtension, SummaryIndexMap, SummaryListItem,
};
use crate::runner::CodeBlockExecutionResult;
use std::ops::Add;

const BASE_HEADER_LEVEL: usize = 1;
const SOURCE_DIRECTORY: &str = "src";
const BOOK_TOML_FILENAME: &str = "book.toml";
pub const SUMMARY_FILENAME: &str = "SUMMARY";
const SHORT_DOCUMENTATION_LEN: usize = 200;
pub const BASE_MODULE_CHAPTER_PREFIX: &str = "##";
pub const GROUP_CHAPTER_PREFIX: &str = "- ###";

/// Prefixes that indicate the start of complex Markdown structures,
/// such as tables. These should be avoided in brief documentation to maintain simple text
/// formatting and prevent disruption of the layout.
const SHORT_DOCUMENTATION_AVOID_PREFIXES: &[&str] =
    &["#", "\n\n", "```", "~~~", "- ", "1.  ", "{{#"];

pub struct MarkdownContent {
    book_toml: String,
    summary: SummaryIndexMap,
    doc_files: Vec<GeneratedFile>,
    pub files_extension: &'static str,
}

impl MarkdownContent {
    pub fn from_crate(
        package_information: &PackageInformation,
        format: OutputFilesExtension,
        execution_results: Option<Vec<CodeBlockExecutionResult>>,
    ) -> Result<Self> {
        let (summary, doc_files) =
            generate_summary_file_content(&package_information.crate_, format, execution_results)?;
        Ok(Self {
            book_toml: generate_book_toml_content(&package_information.metadata),
            summary,
            doc_files,
            files_extension: format.get_string(),
        })
    }
}

/// Builds [`MarkdownContent`] for multiple packages without keeping multiple [`crate::PackageContext`]s
/// or [`PackageInformation`]s items alive simultaneously.
pub struct WorkspaceMarkdownBuilder {
    book_toml: Option<String>,
    summary: SummaryIndexMap,
    doc_files: Vec<GeneratedFile>,
    output_format: OutputFilesExtension,
}

impl WorkspaceMarkdownBuilder {
    pub fn new(output_format: OutputFilesExtension) -> Self {
        Self {
            book_toml: None,
            summary: SummaryIndexMap::new(),
            doc_files: Vec::new(),
            output_format,
        }
    }

    pub fn add_package(&mut self, package_information: &PackageInformation) -> Result<()> {
        if self.book_toml.is_none() {
            self.book_toml = Some(generate_book_toml_content(&package_information.metadata));
        }
        let (summary, files) =
            generate_summary_file_content(&package_information.crate_, self.output_format, None)?;
        let current = std::mem::replace(&mut self.summary, SummaryIndexMap::new());
        self.summary = current.add(summary);
        self.doc_files.extend(files);
        Ok(())
    }

    pub fn build(self) -> Result<MarkdownContent> {
        // TODO(#2790): consider generating book.toml content from workspace metadata
        let book_toml = self
            .book_toml
            .unwrap_or_else(|| generate_book_toml_content(&package_information_placeholder()));
        Ok(MarkdownContent {
            book_toml,
            summary: self.summary,
            doc_files: self.doc_files,
            files_extension: self.output_format.get_string(),
        })
    }
}

fn package_information_placeholder() -> crate::AdditionalMetadata {
    crate::AdditionalMetadata {
        name: "workspace".to_string(),
        authors: None,
    }
}

impl MarkdownContent {
    fn get_summary_markdown(&self) -> String {
        let mut markdown = String::new();
        for (
            md_file_path,
            SummaryListItem {
                chapter,
                nesting_level,
            },
        ) in self.summary.iter()
        {
            markdown.push_str(&format!(
                "{}- [{}]({})\n",
                "  ".repeat(*nesting_level),
                chapter,
                md_file_path
            ));
        }
        markdown
    }

    pub fn save(self, output_dir: &Utf8Path) -> Result<()> {
        let source_directory_path = output_dir.join(SOURCE_DIRECTORY);
        fs::create_dir_all(&source_directory_path)
            .map_err(|e| IODirectoryCreationError::new(e, "generated documentation"))?;

        fs::write(output_dir.join(BOOK_TOML_FILENAME), &self.book_toml)
            .map_err(|e| IOWriteError::new(e, "book.toml"))?;

        fs::write(
            source_directory_path.join(format!("{SUMMARY_FILENAME}{}", self.files_extension)),
            self.get_summary_markdown(),
        )
        .map_err(|e| IOWriteError::new(e, "summary"))?;

        for (filename, file_content) in self.doc_files {
            let path = source_directory_path.join(&filename);
            fs::write(path, file_content).map_err(|e| IOWriteError::new(e, filename.as_ref()))?;
        }

        Ok(())
    }
}

/// Adds order preserving functionality to standard hashmap.
pub struct IndexMap<K, V> {
    map: HashMap<K, (V, usize)>,
    keys: Vec<K>,
}

impl<K, V> IndexMap<K, V>
where
    K: std::hash::Hash + Eq + Clone,
{
    fn new() -> Self {
        IndexMap {
            map: HashMap::new(),
            keys: Vec::new(),
        }
    }

    /// Insert a key-value pair, replaces any existing value for the key.
    fn insert(&mut self, key: K, value: V) {
        if let Some((_, idx)) = self.map.get(&key) {
            self.map.insert(key.clone(), (value, *idx));
        } else {
            let position = self.keys.len();
            self.keys.push(key.clone());
            self.map.insert(key, (value, position));
        }
    }

    /// Iterate over the map in insertion order.
    fn iter(&self) -> impl Iterator<Item = (&K, &V)> {
        self.keys.iter().map(move |k| (k, &self.map[k].0))
    }

    /// Checks if the key exists.
    fn contains_key(&self, key: &K) -> bool {
        self.map.contains_key(key)
    }
}

impl<K, V> Extend<(K, V)> for IndexMap<K, V>
where
    K: std::hash::Hash + Eq + Clone,
{
    fn extend<T: IntoIterator<Item = (K, V)>>(&mut self, iter: T) {
        for (k, v) in iter {
            self.insert(k, v);
        }
    }
}

impl<K, V> Add for IndexMap<K, V>
where
    K: std::hash::Hash + Eq + Clone,
{
    type Output = Self;

    /// Returns a new IndexMap that contains all entries from `self` followed by
    /// all key-value pairs from `rhs` in their original insertion order.
    /// If a key from `rhs` already exists in `self`, its value is replaced while
    /// preserving the original position of the key in `self`.
    fn add(mut self, mut rhs: Self) -> Self::Output {
        // Append in the exact order `rhs` had been built.
        for k in rhs.keys.drain(..) {
            if let Some((v, _)) = rhs.map.remove(&k) {
                self.insert(k, v);
            }
        }
        self
    }
}
