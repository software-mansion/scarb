use anyhow::Result;
use cairo_lang_doc::documentable_item::DocumentableItemId;
use camino::Utf8Path;
use itertools::chain;
use std::collections::{HashMap, HashSet};
use std::fs;

use crate::docs_generation::markdown::book_toml::generate_book_toml_content;
use crate::docs_generation::markdown::summary::generate_summary_file_content;
use crate::docs_generation::markdown::traits::{
    generate_markdown_list_for_top_level_subitems, TopLevelMarkdownDocItem,
};
use crate::docs_generation::{collect_all_top_level_items, TopLevelItems};
use crate::errors::{IODirectoryCreationError, IOWriteError};
use crate::types::{
    Constant, Enum, ExternFunction, ExternType, FreeFunction, Impl, ImplAlias, IncludedItems, Module, Struct, Trait, TypeAlias
};
use crate::PackageInformation;

use super::DocItem;

mod book_toml;
mod summary;
mod traits;

const BASE_HEADER_LEVEL: usize = 1;
const SOURCE_DIRECTORY: &str = "src";
const BOOK_TOML_FILENAME: &str = "book.toml";
pub const SUMMARY_FILENAME: &str = "SUMMARY.md";

type Filename = String;

pub struct MarkdownContent {
    book_toml: String,
    summary: String,
    doc_files: Vec<(Filename, String)>,
}

impl MarkdownContent {
    pub fn from_crate(package_information: &PackageInformation) -> Result<Self> {
        let top_level_items = collect_all_top_level_items(&package_information.crate_);
        let included_items = &package_information.crate_.included_items;

        let summary_file_content = generate_summary_file_content(&top_level_items)?;
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

        let docs_for_top_level_items = chain!(
            generate_top_level_docs_contents(&modules, included_items)?,
            generate_top_level_docs_contents(&constants, included_items)?,
            generate_top_level_docs_contents(&free_functions, included_items)?,
            generate_top_level_docs_contents(&structs, included_items)?,
            generate_top_level_docs_contents(&enums, included_items)?,
            generate_top_level_docs_contents(&type_aliases, included_items)?,
            generate_top_level_docs_contents(&impl_aliases, included_items)?,
            generate_top_level_docs_contents(&traits, included_items)?,
            generate_top_level_docs_contents(&impls, included_items)?,
            generate_top_level_docs_contents(&extern_types, included_items)?,
            generate_top_level_docs_contents(&extern_functions, included_items)?,
        )
        .collect::<Vec<(String, String)>>();

        let summaries_for_top_level_items = vec![
            (
                Module::ITEMS_SUMMARY_FILENAME.to_string(),
                generate_markdown_list_for_top_level_subitems(&modules, BASE_HEADER_LEVEL)?,
            ),
            (
                Constant::ITEMS_SUMMARY_FILENAME.to_string(),
                generate_markdown_list_for_top_level_subitems(&constants, BASE_HEADER_LEVEL)?,
            ),
            (
                FreeFunction::ITEMS_SUMMARY_FILENAME.to_string(),
                generate_markdown_list_for_top_level_subitems(&free_functions, BASE_HEADER_LEVEL)?,
            ),
            (
                Struct::ITEMS_SUMMARY_FILENAME.to_string(),
                generate_markdown_list_for_top_level_subitems(&structs, BASE_HEADER_LEVEL)?,
            ),
            (
                Enum::ITEMS_SUMMARY_FILENAME.to_string(),
                generate_markdown_list_for_top_level_subitems(&enums, BASE_HEADER_LEVEL)?,
            ),
            (
                TypeAlias::ITEMS_SUMMARY_FILENAME.to_string(),
                generate_markdown_list_for_top_level_subitems(&type_aliases, BASE_HEADER_LEVEL)?,
            ),
            (
                ImplAlias::ITEMS_SUMMARY_FILENAME.to_string(),
                generate_markdown_list_for_top_level_subitems(&impl_aliases, BASE_HEADER_LEVEL)?,
            ),
            (
                Trait::ITEMS_SUMMARY_FILENAME.to_string(),
                generate_markdown_list_for_top_level_subitems(&traits, BASE_HEADER_LEVEL)?,
            ),
            (
                Impl::ITEMS_SUMMARY_FILENAME.to_string(),
                generate_markdown_list_for_top_level_subitems(&impls, BASE_HEADER_LEVEL)?,
            ),
            (
                ExternType::ITEMS_SUMMARY_FILENAME.to_string(),
                generate_markdown_list_for_top_level_subitems(&extern_types, BASE_HEADER_LEVEL)?,
            ),
            (
                ExternFunction::ITEMS_SUMMARY_FILENAME.to_string(),
                generate_markdown_list_for_top_level_subitems(
                    &extern_functions,
                    BASE_HEADER_LEVEL,
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
    included_items: &IncludedItems,
) -> Result<Vec<(Filename, String)>> {
    items
        .iter()
        .map(|item| {
            let filename = item.filename();
            item.generate_markdown(included_items, BASE_HEADER_LEVEL)
                .map(|markdown| (filename, markdown))
        })
        .collect()
}
