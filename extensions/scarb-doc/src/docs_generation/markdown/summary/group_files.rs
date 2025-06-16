use crate::docs_generation::TopLevelItems;
use crate::docs_generation::markdown::context::MarkdownGenerationContext;
use crate::docs_generation::markdown::summary::files::{
    generate_doc_files_for_module_items, generate_modules_summary_files,
    generate_summary_files_for_module_items,
};
use crate::docs_generation::markdown::traits::generate_markdown_table_summary_for_top_level_subitems;
use crate::docs_generation::markdown::{GROUP_CHAPTER_PREFIX, SummaryIndexMap};
use crate::types::groups::Group;
use itertools::Itertools;

pub fn generate_global_groups_summary_files(
    groups: &[Group],
    context: &MarkdownGenerationContext,
    summary_index_map: &SummaryIndexMap,
) -> anyhow::Result<Vec<(String, String)>> {
    let mut doc_files: Vec<(String, String)> = Vec::new();

    if !groups.is_empty() {
        for group in groups.iter() {
            let mut top_level_items = TopLevelItems::default();
            let Group {
                name: _,
                submodules,
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
            } = &group;

            top_level_items.modules.extend(submodules);
            top_level_items.constants.extend(constants);
            top_level_items.free_functions.extend(free_functions);
            top_level_items.structs.extend(structs);
            top_level_items.enums.extend(enums);
            top_level_items.type_aliases.extend(type_aliases);
            top_level_items.impl_aliases.extend(impl_aliases);
            top_level_items.traits.extend(traits);
            top_level_items.impls.extend(impls);
            top_level_items.extern_types.extend(extern_types);
            top_level_items.extern_functions.extend(extern_functions);

            doc_files.extend(generate_summary_files_for_module_items(
                &top_level_items,
                group.get_name_normalized(),
                context,
            )?);

            doc_files.extend(
                generate_doc_files_for_module_items(&top_level_items, context, summary_index_map)?
                    .to_owned(),
            );

            doc_files.push((
                group.filename(),
                generate_markdown_for_group(group, context)?,
            ));

            if !top_level_items.modules.is_empty() {
                for submodule in group.submodules.iter() {
                    let sub_summaries =
                        &generate_modules_summary_files(submodule, context, summary_index_map)?;
                    doc_files.extend::<Vec<(String, String)>>(sub_summaries.to_owned());
                }
            };
        }
    }
    Ok(doc_files)
}

pub fn generate_markdown_for_group(
    group: &Group,
    context: &MarkdownGenerationContext,
) -> anyhow::Result<String> {
    let mut markdown = format!("\n# {}\n", group.name).to_string();

    let markdown_formatted_path = group.get_name_normalized();
    markdown += &generate_markdown_table_summary_for_top_level_subitems(
        &group.submodules.iter().collect_vec(),
        context,
        &markdown_formatted_path,
        GROUP_CHAPTER_PREFIX,
    )?;
    markdown += &generate_markdown_table_summary_for_top_level_subitems(
        &group.constants.iter().collect_vec(),
        context,
        &markdown_formatted_path,
        GROUP_CHAPTER_PREFIX,
    )?;
    markdown += &generate_markdown_table_summary_for_top_level_subitems(
        &group.free_functions.iter().collect_vec(),
        context,
        &markdown_formatted_path,
        GROUP_CHAPTER_PREFIX,
    )?;
    markdown += &generate_markdown_table_summary_for_top_level_subitems(
        &group.structs.iter().collect_vec(),
        context,
        &markdown_formatted_path,
        GROUP_CHAPTER_PREFIX,
    )?;
    markdown += &generate_markdown_table_summary_for_top_level_subitems(
        &group.enums.iter().collect_vec(),
        context,
        &markdown_formatted_path,
        GROUP_CHAPTER_PREFIX,
    )?;
    markdown += &generate_markdown_table_summary_for_top_level_subitems(
        &group.type_aliases.iter().collect_vec(),
        context,
        &markdown_formatted_path,
        GROUP_CHAPTER_PREFIX,
    )?;
    markdown += &generate_markdown_table_summary_for_top_level_subitems(
        &group.impl_aliases.iter().collect_vec(),
        context,
        &markdown_formatted_path,
        GROUP_CHAPTER_PREFIX,
    )?;
    markdown += &generate_markdown_table_summary_for_top_level_subitems(
        &group.traits.iter().collect_vec(),
        context,
        &markdown_formatted_path,
        GROUP_CHAPTER_PREFIX,
    )?;
    markdown += &generate_markdown_table_summary_for_top_level_subitems(
        &group.impls.iter().collect_vec(),
        context,
        &markdown_formatted_path,
        GROUP_CHAPTER_PREFIX,
    )?;
    markdown += &generate_markdown_table_summary_for_top_level_subitems(
        &group.extern_types.iter().collect_vec(),
        context,
        &markdown_formatted_path,
        GROUP_CHAPTER_PREFIX,
    )?;
    markdown += &generate_markdown_table_summary_for_top_level_subitems(
        &group.extern_functions.iter().collect_vec(),
        context,
        &markdown_formatted_path,
        GROUP_CHAPTER_PREFIX,
    )?;

    Ok(markdown)
}
