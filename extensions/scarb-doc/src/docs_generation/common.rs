use crate::docs_generation::markdown::IndexMap;
use scarb_extensions_cli::doc::OutputFormat;
use std::sync::OnceLock;

pub type Filename = String;
pub type GeneratedFile = (Filename, String);

/// Stores `SUMMARY.md` files data: filepath, chapter name and list indent.
pub type SummaryIndexMap = IndexMap<String, SummaryListItem>;

pub struct SummaryListItem {
    /// A SUMMARY.md chapter title.
    pub chapter: String,
    /// List item indent in the SUMMARY.md file.
    pub nesting_level: usize,
}

impl SummaryListItem {
    pub fn new(chapter: String, nesting_level: usize) -> Self {
        Self {
            chapter,
            nesting_level,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFilesExtension {
    Md,
    Mdx,
    Json,
}

impl OutputFilesExtension {
    pub const fn get_string(&self) -> &'static str {
        match self {
            OutputFilesExtension::Md => ".md",
            OutputFilesExtension::Mdx => ".mdx",
            OutputFilesExtension::Json => ".json",
        }
    }
}

impl From<OutputFormat> for OutputFilesExtension {
    fn from(format: OutputFormat) -> Self {
        match format {
            OutputFormat::Markdown => OutputFilesExtension::Md,
            OutputFormat::Mdx => OutputFilesExtension::Mdx,
            OutputFormat::Json => OutputFilesExtension::Json,
        }
    }
}

// Global, run-scoped output extension accessor.
static OUTPUT_EXTENSION: OnceLock<&'static str> = OnceLock::new();

pub fn set_output_extension(ext: OutputFormat) {
    let _ = OUTPUT_EXTENSION.set(OutputFilesExtension::from(ext).get_string());
}

fn extension() -> &'static str {
    OUTPUT_EXTENSION.get().copied().unwrap()
}

pub fn get_filename_with_extension(filename: &str) -> String {
    format!("{filename}{}", extension())
}
