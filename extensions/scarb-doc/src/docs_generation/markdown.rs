use anyhow::Result;
use camino::Utf8Path;
use context::MarkdownGenerationContext;
use itertools::chain;
use std::fs;

use crate::PackageInformation;
use crate::docs_generation::markdown::book_toml::generate_book_toml_content;
use crate::docs_generation::markdown::summary::generate_summary_file_content;
use crate::docs_generation::markdown::traits::{
    TopLevelMarkdownDocItem, generate_markdown_table_summary_for_top_level_subitems,
};
use crate::docs_generation::{TopLevelItems, collect_all_top_level_items};
use crate::errors::{IODirectoryCreationError, IOWriteError};
use crate::types::{
    Constant, Enum, ExternFunction, ExternType, FreeFunction, Impl, ImplAlias, Module, Struct,
    Trait, TypeAlias,
};

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
        let top_level_items = collect_all_top_level_items(&package_information.crate_);

        let summary_file_content = generate_summary_file_content(
            &package_information.crate_.root_module,
            &top_level_items,
        )?;
        let TopLevelItems {
            modules,
            constants,
            free_functions,
            structs,
            enums,
            type_aliases,
            impl_aliases,
            traits,
            impls,
            extern_types,
            extern_functions,
        } = top_level_items;

        let context = MarkdownGenerationContext::from_crate(&package_information.crate_);
        let docs_for_top_level_items = chain!(
            generate_top_level_docs_contents(&modules, &context)?,
            generate_top_level_docs_contents(&constants, &context)?,
            generate_top_level_docs_contents(&free_functions, &context)?,
            generate_top_level_docs_contents(&structs, &context)?,
            generate_top_level_docs_contents(&enums, &context)?,
            generate_top_level_docs_contents(&type_aliases, &context)?,
            generate_top_level_docs_contents(&impl_aliases, &context)?,
            generate_top_level_docs_contents(&traits, &context)?,
            generate_top_level_docs_contents(&impls, &context)?,
            generate_top_level_docs_contents(&extern_types, &context)?,
            generate_top_level_docs_contents(&extern_functions, &context)?,
        )
        .collect::<Vec<(String, String)>>();

        let summaries_for_top_level_items = vec![
            (
                Module::ITEMS_SUMMARY_FILENAME.to_string(),
                generate_markdown_table_summary_for_top_level_subitems(&modules, &context)?,
            ),
            (
                Constant::ITEMS_SUMMARY_FILENAME.to_string(),
                generate_markdown_table_summary_for_top_level_subitems(&constants, &context)?,
            ),
            (
                FreeFunction::ITEMS_SUMMARY_FILENAME.to_string(),
                generate_markdown_table_summary_for_top_level_subitems(&free_functions, &context)?,
            ),
            (
                Struct::ITEMS_SUMMARY_FILENAME.to_string(),
                generate_markdown_table_summary_for_top_level_subitems(&structs, &context)?,
            ),
            (
                Enum::ITEMS_SUMMARY_FILENAME.to_string(),
                generate_markdown_table_summary_for_top_level_subitems(&enums, &context)?,
            ),
            (
                TypeAlias::ITEMS_SUMMARY_FILENAME.to_string(),
                generate_markdown_table_summary_for_top_level_subitems(&type_aliases, &context)?,
            ),
            (
                ImplAlias::ITEMS_SUMMARY_FILENAME.to_string(),
                generate_markdown_table_summary_for_top_level_subitems(&impl_aliases, &context)?,
            ),
            (
                Trait::ITEMS_SUMMARY_FILENAME.to_string(),
                generate_markdown_table_summary_for_top_level_subitems(&traits, &context)?,
            ),
            (
                Impl::ITEMS_SUMMARY_FILENAME.to_string(),
                generate_markdown_table_summary_for_top_level_subitems(&impls, &context)?,
            ),
            (
                ExternType::ITEMS_SUMMARY_FILENAME.to_string(),
                generate_markdown_table_summary_for_top_level_subitems(&extern_types, &context)?,
            ),
            (
                ExternFunction::ITEMS_SUMMARY_FILENAME.to_string(),
                generate_markdown_table_summary_for_top_level_subitems(
                    &extern_functions,
                    &context,
                )?,
            ),
        ]
        .into_iter()
        .filter(|(_filename, content)| !content.is_empty())
        .collect::<Vec<_>>();

        Ok(Self {
            book_toml: generate_book_toml_content(&package_information.metadata),
            summary: summary_file_content,
            doc_files: chain!(docs_for_top_level_items, summaries_for_top_level_items).collect(),
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

fn generate_top_level_docs_contents(
    items: &[&impl TopLevelMarkdownDocItem],
    context: &MarkdownGenerationContext,
) -> Result<Vec<(Filename, String)>> {
    items
        .iter()
        .map(|item| {
            let filename = item.filename();
            item.generate_markdown(context, BASE_HEADER_LEVEL)
                .map(|markdown| (filename, markdown))
        })
        .collect()
}
