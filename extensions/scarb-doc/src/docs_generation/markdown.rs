use crate::PackageInformation;
use crate::docs_generation::markdown::book_toml::generate_book_toml_content;
use crate::docs_generation::markdown::summary::generate_summary_file_content;
use crate::errors::{IODirectoryCreationError, IOWriteError};
use anyhow::Result;
use camino::Utf8Path;
use std::collections::HashMap;
use std::fs;

mod book_toml;
mod context;
mod summary;
mod traits;

const BASE_HEADER_LEVEL: usize = 1;
const SOURCE_DIRECTORY: &str = "src";
const BOOK_TOML_FILENAME: &str = "book.toml";
pub const SUMMARY_FILENAME: &str = "SUMMARY.md";
const SHORT_DOCUMENTATION_LEN: usize = 200;
pub const BASE_MODULE_CHAPTER_PREFIX: &str = "##";
pub const GROUP_CHAPTER_PREFIX: &str = "- ###";

/// Prefixes that indicate the start of complex markdown structures,
/// such as tables. These should be avoided in brief documentation to maintain simple text
/// formatting and prevent disruption of the layout.
const SHORT_DOCUMENTATION_AVOID_PREFIXES: &[&str] = &["#", "\n\n", "```\n", "- ", "1.  "];

type Filename = String;
type GeneratedFile = (Filename, String);

/// Stores `SUMMARY.md` files data: filepath, chapter name and list indent.
pub type SummaryIndexMap = IndexMap<String, SummaryListItem>;

pub struct SummaryListItem {
    /// A SUMMARY.md chapter title.  
    chapter: String,
    /// List item indent in SUMMARY.md file.
    nesting_level: usize,
}

impl SummaryListItem {
    pub fn new(chapter: String, nesting_level: usize) -> Self {
        Self {
            chapter,
            nesting_level,
        }
    }
}

pub struct MarkdownContent {
    book_toml: String,
    summary: SummaryIndexMap,
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

    fn get_summary_markdown(&self) -> String {
        let mut markdown = "# Summary\n\n".to_string();
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
            source_directory_path.join(SUMMARY_FILENAME),
            self.get_summary_markdown(),
        )
        .map_err(|e| IOWriteError::new(e, "summary"))?;

        for (filename, file_content) in self.doc_files {
            fs::write(source_directory_path.join(filename.clone()), file_content)
                .map_err(|e| IOWriteError::new(e, filename.as_ref()))?;
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
    pub fn contains_key(&self, key: &K) -> bool {
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
