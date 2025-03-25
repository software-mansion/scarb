use anyhow::Result;
use std::fmt::Write;

use super::traits::mark_duplicated_item_with_relative_path;
use crate::docs_generation::markdown::context::path_to_file_link;
use crate::docs_generation::markdown::traits::TopLevelMarkdownDocItem;
use crate::docs_generation::{DocItem, TopLevelItems};
use crate::types::Module;

pub fn generate_summary_file_content(
    root_module: &Module,
    top_level_items: &TopLevelItems,
) -> Result<String> {
    let mut markdown = format!(
        "# Summary\n\n---\n- [{}](./{})\n",
        Module::HEADER,
        Module::ITEMS_SUMMARY_FILENAME
    );
    markdown += &generate_modules_summary_content(root_module, 1)?;

    let TopLevelItems {
        modules: _,
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

    markdown += &generate_markdown_list_summary_for_top_level_subitems(constants)?;
    markdown += &generate_markdown_list_summary_for_top_level_subitems(free_functions)?;
    markdown += &generate_markdown_list_summary_for_top_level_subitems(structs)?;
    markdown += &generate_markdown_list_summary_for_top_level_subitems(enums)?;
    markdown += &generate_markdown_list_summary_for_top_level_subitems(type_aliases)?;
    markdown += &generate_markdown_list_summary_for_top_level_subitems(impl_aliases)?;
    markdown += &generate_markdown_list_summary_for_top_level_subitems(traits)?;
    markdown += &generate_markdown_list_summary_for_top_level_subitems(impls)?;
    markdown += &generate_markdown_list_summary_for_top_level_subitems(extern_types)?;
    markdown += &generate_markdown_list_summary_for_top_level_subitems(extern_functions)?;

    Ok(markdown)
}

fn generate_markdown_list_summary_for_top_level_subitems<T: TopLevelMarkdownDocItem>(
    subitems: &[&T],
) -> Result<String> {
    let mut markdown = String::new();
    if !subitems.is_empty() {
        writeln!(
            &mut markdown,
            "---\n- [{}](./{})",
            T::HEADER,
            T::ITEMS_SUMMARY_FILENAME
        )?;
        let items_with_relative_path = mark_duplicated_item_with_relative_path(subitems);
        for (item, relative_path) in items_with_relative_path {
            writeln!(
                &mut markdown,
                "  {}",
                item.generate_markdown_list_item(relative_path)
            )?;
        }
    }

    Ok(markdown)
}

pub fn generate_modules_summary_content(
    module: &Module,
    mut nesting_level: usize,
) -> Result<String> {
    let mut markdown = String::new();
    writeln!(
        markdown,
        "{}- [{}]({})",
        "  ".repeat(nesting_level),
        module.item_data.name,
        path_to_file_link(module.full_path())
    )?;
    nesting_level += 1;
    for submodule in module.submodules.iter() {
        markdown += &generate_modules_summary_content(submodule, nesting_level)?;
    }

    Ok(markdown.to_string())
}
