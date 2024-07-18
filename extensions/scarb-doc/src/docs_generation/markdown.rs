use anyhow::{Context, Result};
use camino::Utf8Path;
use itertools::chain;
use std::fs;

use crate::docs_generation::markdown::book_toml::generate_book_toml_content;
use crate::docs_generation::markdown::summary::generate_summary_file_content;
use crate::docs_generation::markdown::traits::TopLevelMarkdownDocItem;
use crate::docs_generation::{collect_all_top_level_items, TopLevelItems};
use crate::PackageInformation;

mod book_toml;
mod summary;
mod traits;

const BASE_HEADER_LEVEL: usize = 1;
const SOURCE_DIRECTORY: &str = "src";
const BOOK_TOML_FILENAME: &str = "book.toml";
const SUMMARY_FILENAME: &str = "SUMMARY.md";

pub struct MarkdownContent<'a> {
    book_toml: String,
    summary: String,
    top_level_docs: Vec<(&'a dyn TopLevelMarkdownDocItem, String)>,
}

impl<'a> MarkdownContent<'a> {
    pub fn from_crate(package_information: &'a PackageInformation) -> Self {
        let top_level_items = collect_all_top_level_items(&package_information.crate_);

        let TopLevelItems {
            ref modules,
            ref constants,
            ref free_functions,
            ref structs,
            ref enums,
            ref type_aliases,
            ref impl_aliases,
            ref traits,
            ref impls,
            ref extern_types,
            ref extern_functions,
        } = top_level_items;

        let top_level_docs = chain!(
            generate_top_level_docs_contents(modules),
            generate_top_level_docs_contents(constants),
            generate_top_level_docs_contents(free_functions),
            generate_top_level_docs_contents(structs),
            generate_top_level_docs_contents(enums),
            generate_top_level_docs_contents(type_aliases),
            generate_top_level_docs_contents(impl_aliases),
            generate_top_level_docs_contents(traits),
            generate_top_level_docs_contents(impls),
            generate_top_level_docs_contents(extern_types),
            generate_top_level_docs_contents(extern_functions),
        )
        .collect();

        Self {
            book_toml: generate_book_toml_content(&package_information.metadata),
            summary: generate_summary_file_content(&top_level_items),
            top_level_docs,
        }
    }

    pub fn save(self, output_dir: &Utf8Path) -> Result<()> {
        let source_directory_path = output_dir.join(SOURCE_DIRECTORY);
        fs::create_dir_all(&source_directory_path)
            .context("failed to create directory for docs")?;

        fs::write(output_dir.join(BOOK_TOML_FILENAME), self.book_toml)
            .context("failed to write book.toml content to a file")?;

        fs::write(source_directory_path.join(SUMMARY_FILENAME), self.summary)
            .context("failed to write summary content to a file")?;

        for (item, file_content) in self.top_level_docs {
            fs::write(source_directory_path.join(item.filename()), file_content)
                .context("failed to write content to a file")?;
        }

        Ok(())
    }
}

fn generate_top_level_docs_contents<'a>(
    items: &[&'a impl TopLevelMarkdownDocItem],
) -> Vec<(&'a dyn TopLevelMarkdownDocItem, String)> {
    items
        .iter()
        .map(|item| {
            (
                *item as &dyn TopLevelMarkdownDocItem,
                item.generate_markdown(BASE_HEADER_LEVEL),
            )
        })
        .collect()
}
