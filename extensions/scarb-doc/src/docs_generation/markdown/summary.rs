use anyhow::Result;
use std::fmt::Write;

use crate::docs_generation::markdown::traits::TopLevelMarkdownDocItem;
use crate::docs_generation::markdown::BASE_HEADER_LEVEL;
use crate::docs_generation::TopLevelItems;

pub fn generate_summary_file_content(top_level_items: &TopLevelItems) -> Result<String> {
    let header = str::repeat("#", BASE_HEADER_LEVEL);

    let mut markdown = format!("{header} Summary\n\n");

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

    markdown += &generate_markdown_list_summary_for_top_level_subitems(modules)?;
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
            "- [{}](./{})\n",
            T::HEADER,
            T::ITEMS_SUMMARY_FILENAME
        )?;
        for item in subitems {
            writeln!(&mut markdown, "  {}", item.generate_markdown_list_item())?;
        }
    }

    Ok(markdown)
}
