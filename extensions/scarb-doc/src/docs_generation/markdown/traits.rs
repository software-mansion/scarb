use anyhow::Result;
use itertools::Itertools;
use std::collections::HashMap;
use std::fmt::Write;

use crate::docs_generation::{DocItem, PrimitiveDocItem, TopLevelDocItem};
use crate::types::{
    Constant, Enum, ExternFunction, ExternType, FreeFunction, Impl, ImplAlias, Module, Struct,
    Trait, TypeAlias,
};

pub trait TopLevelMarkdownDocItem: MarkdownDocItem + TopLevelDocItem {
    const ITEMS_SUMMARY_FILENAME: &'static str;

    fn filename(&self) -> String {
        format!("{}.md", self.full_path().replace("::", "-"))
    }

    fn md_ref(&self, relative_path: Option<String>) -> String {
        match relative_path {
            Some(path) => format!("[{}](./{})", path, self.filename()),
            None => format!("[{}](./{})", self.name(), self.filename()),
        }
    }

    fn generate_markdown_list_item(&self, relative_path: Option<String>) -> String {
        format!("- {}\n", self.md_ref(relative_path))
    }
}

macro_rules! impl_top_level_markdown_doc_item {
    ($t:ty, $filename:expr) => {
        impl TopLevelMarkdownDocItem for $t {
            const ITEMS_SUMMARY_FILENAME: &'static str = $filename;
        }
    };
}

impl_top_level_markdown_doc_item!(Constant, "constants.md");
impl_top_level_markdown_doc_item!(Enum, "enums.md");
impl_top_level_markdown_doc_item!(ExternFunction, "extern_functions.md");
impl_top_level_markdown_doc_item!(ExternType, "extern_types.md");
impl_top_level_markdown_doc_item!(FreeFunction, "free_functions.md");
impl_top_level_markdown_doc_item!(Impl, "impls.md");
impl_top_level_markdown_doc_item!(ImplAlias, "impl_aliases.md");
impl_top_level_markdown_doc_item!(Module, "modules.md");
impl_top_level_markdown_doc_item!(Struct, "structs.md");
impl_top_level_markdown_doc_item!(Trait, "traits.md");
impl_top_level_markdown_doc_item!(TypeAlias, "type_aliases.md");

pub trait MarkdownDocItem: DocItem {
    fn generate_markdown(&self, header_level: usize) -> Result<String>;
}

impl<T> MarkdownDocItem for T
where
    T: PrimitiveDocItem,
{
    fn generate_markdown(&self, header_level: usize) -> Result<String> {
        generate_markdown_from_item_data(self, header_level)
    }
}

impl MarkdownDocItem for Enum {
    fn generate_markdown(&self, header_level: usize) -> Result<String> {
        let mut markdown = generate_markdown_from_item_data(self, header_level)?;

        markdown += &generate_markdown_for_subitems(&self.variants, header_level)?;

        Ok(markdown)
    }
}

impl MarkdownDocItem for Impl {
    fn generate_markdown(&self, header_level: usize) -> Result<String> {
        let mut markdown = generate_markdown_from_item_data(self, header_level)?;

        markdown += &generate_markdown_for_subitems(&self.impl_constants, header_level)?;
        markdown += &generate_markdown_for_subitems(&self.impl_functions, header_level)?;
        markdown += &generate_markdown_for_subitems(&self.impl_types, header_level)?;

        Ok(markdown)
    }
}

impl MarkdownDocItem for Module {
    fn generate_markdown(&self, header_level: usize) -> Result<String> {
        let mut markdown = generate_markdown_from_item_data(self, header_level)?;

        markdown += &generate_markdown_list_for_top_level_subitems(
            &self.submodules.iter().collect_vec(),
            header_level + 1,
        )?;
        markdown += &generate_markdown_list_for_top_level_subitems(
            &self.constants.iter().collect_vec(),
            header_level + 1,
        )?;
        markdown += &generate_markdown_list_for_top_level_subitems(
            &self.free_functions.iter().collect_vec(),
            header_level + 1,
        )?;
        markdown += &generate_markdown_list_for_top_level_subitems(
            &self.structs.iter().collect_vec(),
            header_level + 1,
        )?;
        markdown += &generate_markdown_list_for_top_level_subitems(
            &self.enums.iter().collect_vec(),
            header_level + 1,
        )?;
        markdown += &generate_markdown_list_for_top_level_subitems(
            &self.type_aliases.iter().collect_vec(),
            header_level + 1,
        )?;
        markdown += &generate_markdown_list_for_top_level_subitems(
            &self.impl_aliases.iter().collect_vec(),
            header_level + 1,
        )?;
        markdown += &generate_markdown_list_for_top_level_subitems(
            &self.traits.iter().collect_vec(),
            header_level + 1,
        )?;
        markdown += &generate_markdown_list_for_top_level_subitems(
            &self.impls.iter().collect_vec(),
            header_level + 1,
        )?;
        markdown += &generate_markdown_list_for_top_level_subitems(
            &self.extern_types.iter().collect_vec(),
            header_level + 1,
        )?;
        markdown += &generate_markdown_list_for_top_level_subitems(
            &self.extern_functions.iter().collect_vec(),
            header_level + 1,
        )?;

        Ok(markdown)
    }
}

impl MarkdownDocItem for Struct {
    fn generate_markdown(&self, header_level: usize) -> Result<String> {
        let mut markdown = generate_markdown_from_item_data(self, header_level)?;

        markdown += &generate_markdown_for_subitems(&self.members, header_level)?;

        Ok(markdown)
    }
}

impl MarkdownDocItem for Trait {
    fn generate_markdown(&self, header_level: usize) -> Result<String> {
        let mut markdown = generate_markdown_from_item_data(self, header_level)?;

        markdown += &generate_markdown_for_subitems(&self.trait_constants, header_level)?;
        markdown += &generate_markdown_for_subitems(&self.trait_functions, header_level)?;
        markdown += &generate_markdown_for_subitems(&self.trait_types, header_level)?;

        Ok(markdown)
    }
}

/// Takes items, and appends for each of them a path, that was trimmed based on the common prefix of all of the items,
/// cthat share the same name.
pub fn mark_duplicated_item_with_relative_path<'a, T: TopLevelMarkdownDocItem + 'a>(
    items: &'a [&'a T],
) -> Vec<(&&'a T, Option<String>)> {
    let mut paths_for_item_name = HashMap::<String, Vec<String>>::new();
    for item in items {
        paths_for_item_name
            .entry(item.name().to_string())
            .or_default()
            .push(item.name().to_string());
    }

    let common_path_prefix_lengths_for_item: HashMap<String, usize> = paths_for_item_name
        .iter()
        .filter(|(_, paths)| paths.len() > 1)
        .map(|(name, paths)| {
            let splitted_paths: Vec<Vec<String>> = paths
                .iter()
                .map(|path| path.split("::").map(|val| val.to_string()).collect())
                .collect();

            let min_len = splitted_paths
                .iter()
                .map(|vec| vec.len())
                .min()
                .unwrap_or(0);

            let mut prefix_len = min_len;
            for i in 0..min_len {
                let first = &splitted_paths[0][i];
                if !splitted_paths.iter().all(|vec| &vec[i] == first) {
                    prefix_len = i;
                    break;
                }
            }

            (name.clone(), prefix_len)
        })
        .collect();

    items
        .iter()
        .map(|item| {
            let relative_path =
                common_path_prefix_lengths_for_item
                    .get(item.name())
                    .map(|common_prefix_length| {
                        item.full_path()
                            .split("::")
                            .skip(*common_prefix_length)
                            .collect::<Vec<_>>()
                            .join("::")
                    });
            (item, relative_path)
        })
        .collect::<Vec<_>>()
}

pub fn generate_markdown_list_for_top_level_subitems<T: TopLevelMarkdownDocItem>(
    subitems: &[&T],
    header_level: usize,
) -> Result<String> {
    let mut markdown = String::new();

    if !subitems.is_empty() {
        let header = str::repeat("#", header_level);

        writeln!(&mut markdown, "{header} {}\n", T::HEADER)?;
        let items_with_relative_path = mark_duplicated_item_with_relative_path(subitems);
        for (item, relative_path) in items_with_relative_path {
            writeln!(
                &mut markdown,
                "{}",
                item.generate_markdown_list_item(relative_path)
            )?;
        }
    }

    Ok(markdown)
}

fn generate_markdown_for_subitems<T: MarkdownDocItem + PrimitiveDocItem>(
    subitems: &[T],
    header_level: usize,
) -> Result<String> {
    let mut markdown = String::new();

    if !subitems.is_empty() {
        let header = str::repeat("#", header_level + 1);

        writeln!(&mut markdown, "{header} {}\n", T::HEADER)?;
        for item in subitems {
            writeln!(
                &mut markdown,
                "{}",
                item.generate_markdown(header_level + 2)?
            )?;
        }
    }

    Ok(markdown)
}

fn generate_markdown_from_item_data(
    doc_item: &impl DocItem,
    header_level: usize,
) -> Result<String> {
    let mut markdown = String::new();

    let header = str::repeat("#", header_level);

    writeln!(&mut markdown, "{header} {}\n", doc_item.name())?;

    if let Some(doc) = doc_item.doc() {
        writeln!(&mut markdown, "{doc}\n")?;
    }

    writeln!(
        &mut markdown,
        "Fully qualified path: `{}`\n",
        doc_item.full_path()
    )?;

    if let Some(sig) = &doc_item.signature() {
        if !sig.is_empty() {
            // TODO(#1525) add cairo support to mdbook
            writeln!(&mut markdown, "```rust\n{sig}\n```\n")?;
        }
    }

    Ok(markdown)
}
