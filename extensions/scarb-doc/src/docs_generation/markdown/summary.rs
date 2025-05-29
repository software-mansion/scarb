use super::traits::{
    MarkdownDocItem, generate_markdown_table_summary_for_top_level_subitems,
    mark_duplicated_item_with_relative_path,
};
use crate::docs_generation::markdown::context::{MarkdownGenerationContext, path_to_file_link};
use crate::docs_generation::markdown::groups::generate_global_groups_summary_content;
use crate::docs_generation::markdown::traits::TopLevelMarkdownDocItem;
use crate::docs_generation::markdown::{BASE_HEADER_LEVEL, BASE_MODULE_CHAPTER_PREFIX, Filename};
use crate::docs_generation::{DocItem, TopLevelItems};
use crate::types::crate_type::Crate;
use crate::types::module_type::Module;
use crate::types::other_types::{
    Constant, Enum, ExternFunction, ExternType, FreeFunction, Impl, ImplAlias, Struct, Trait,
    TypeAlias,
};
use anyhow::Result;
use itertools::chain;
use std::fmt::Write;

pub fn generate_summary_file_content(crate_: &Crate) -> Result<(String, Vec<(String, String)>)> {
    let mut markdown = "# Summary\n\n".to_string();
    let context = MarkdownGenerationContext::from_crate(crate_);

    let mut summary_files = vec![(
        crate_.root_module.filename(),
        crate_
            .root_module
            .generate_markdown(&context, BASE_HEADER_LEVEL, None)?,
    )];
    let (sub_markdown, module_item_summaries) =
        &generate_modules_summary_content(&crate_.root_module, 0, &context)?;
    markdown += sub_markdown;
    summary_files.extend(module_item_summaries.to_owned());

    let (groups_markdown, groups_files) =
        generate_global_groups_summary_content(&crate_.groups, &context)?;
    markdown += &groups_markdown;
    summary_files.extend(groups_files.to_owned());

    Ok((markdown, summary_files))
}

pub fn generate_modules_summary_content(
    module: &Module,
    mut nesting_level: usize,
    context: &MarkdownGenerationContext,
) -> Result<(String, Vec<(String, String)>)> {
    let mut markdown = String::new();
    writeln!(
        markdown,
        "{}- [{}]({})",
        "  ".repeat(nesting_level),
        module.item_data.name,
        path_to_file_link(module.full_path())
    )?;

    let mut top_level_items = TopLevelItems::default();
    let Module {
        module_id: _module_id,
        item_data: _item_data,
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
        ..
    } = &module;

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

    let mut doc_files = generate_summary_files_for_module_items(
        &top_level_items,
        module.markdown_formatted_path(),
        context,
    )?;

    doc_files.extend::<Vec<(String, String)>>(
        generate_doc_files_for_module_items(&top_level_items, context)?.to_owned(),
    );

    nesting_level += 1;
    if !top_level_items.modules.is_empty() {
        writeln!(
            &mut markdown,
            "{}- [{}](./{}-{})",
            "  ".repeat(nesting_level),
            Module::HEADER,
            module.markdown_formatted_path(),
            Module::ITEMS_SUMMARY_FILENAME
        )?;
        nesting_level += 1;
        for submodule in module.submodules.iter() {
            let (sub_markdown, sub_summaries) =
                &generate_modules_summary_content(submodule, nesting_level, context)?;
            markdown += sub_markdown;
            doc_files.extend::<Vec<(String, String)>>(sub_summaries.to_owned());
        }
        nesting_level -= 1;
    }

    markdown += &generate_markdown_list_summary_for_module_items(
        &top_level_items.constants,
        nesting_level,
        &module.markdown_formatted_path(),
    )?;
    markdown += &generate_markdown_list_summary_for_module_items(
        &top_level_items.free_functions,
        nesting_level,
        &module.markdown_formatted_path(),
    )?;
    markdown += &generate_markdown_list_summary_for_module_items(
        &top_level_items.structs,
        nesting_level,
        &module.markdown_formatted_path(),
    )?;
    markdown += &generate_markdown_list_summary_for_module_items(
        &top_level_items.enums,
        nesting_level,
        &module.markdown_formatted_path(),
    )?;
    markdown += &generate_markdown_list_summary_for_module_items(
        &top_level_items.type_aliases,
        nesting_level,
        &module.markdown_formatted_path(),
    )?;
    markdown += &generate_markdown_list_summary_for_module_items(
        &top_level_items.impl_aliases,
        nesting_level,
        &module.markdown_formatted_path(),
    )?;
    markdown += &generate_markdown_list_summary_for_module_items(
        &top_level_items.traits,
        nesting_level,
        &module.markdown_formatted_path(),
    )?;
    markdown += &generate_markdown_list_summary_for_module_items(
        &top_level_items.impls,
        nesting_level,
        &module.markdown_formatted_path(),
    )?;
    markdown += &generate_markdown_list_summary_for_module_items(
        &top_level_items.extern_types,
        nesting_level,
        &module.markdown_formatted_path(),
    )?;
    markdown += &generate_markdown_list_summary_for_module_items(
        &top_level_items.extern_functions,
        nesting_level,
        &module.markdown_formatted_path(),
    )?;
    Ok((markdown.to_string(), doc_files))
}

pub fn generate_doc_files_for_module_items(
    top_level_items: &TopLevelItems,
    context: &MarkdownGenerationContext,
) -> Result<Vec<(String, String)>> {
    Ok(chain!(
        generate_top_level_docs_contents(&top_level_items.modules, context)?,
        generate_top_level_docs_contents(&top_level_items.constants, context)?,
        generate_top_level_docs_contents(&top_level_items.free_functions, context)?,
        generate_top_level_docs_contents(&top_level_items.structs, context)?,
        generate_top_level_docs_contents(&top_level_items.enums, context)?,
        generate_top_level_docs_contents(&top_level_items.type_aliases, context)?,
        generate_top_level_docs_contents(&top_level_items.impl_aliases, context)?,
        generate_top_level_docs_contents(&top_level_items.traits, context)?,
        generate_top_level_docs_contents(&top_level_items.impls, context)?,
        generate_top_level_docs_contents(&top_level_items.extern_types, context)?,
        generate_top_level_docs_contents(&top_level_items.extern_functions, context)?,
    )
    .collect::<Vec<(String, String)>>())
}

pub fn generate_summary_files_for_module_items(
    top_level_items: &TopLevelItems,
    module_name: String,
    context: &MarkdownGenerationContext,
) -> Result<Vec<(String, String)>> {
    Ok(vec![
        (
            format!("{}-{}", module_name, Module::ITEMS_SUMMARY_FILENAME),
            generate_markdown_table_summary_for_top_level_subitems(
                &top_level_items.modules,
                context,
                &module_name,
                BASE_MODULE_CHAPTER_PREFIX,
            )?,
        ),
        (
            format!("{}-{}", module_name, Constant::ITEMS_SUMMARY_FILENAME),
            generate_markdown_table_summary_for_top_level_subitems(
                &top_level_items.constants,
                context,
                &module_name,
                BASE_MODULE_CHAPTER_PREFIX,
            )?,
        ),
        (
            format!("{}-{}", module_name, FreeFunction::ITEMS_SUMMARY_FILENAME),
            generate_markdown_table_summary_for_top_level_subitems(
                &top_level_items.free_functions,
                context,
                &module_name,
                BASE_MODULE_CHAPTER_PREFIX,
            )?,
        ),
        (
            format!("{}-{}", module_name, Struct::ITEMS_SUMMARY_FILENAME),
            generate_markdown_table_summary_for_top_level_subitems(
                &top_level_items.structs,
                context,
                &module_name,
                BASE_MODULE_CHAPTER_PREFIX,
            )?,
        ),
        (
            format!("{}-{}", module_name, Enum::ITEMS_SUMMARY_FILENAME),
            generate_markdown_table_summary_for_top_level_subitems(
                &top_level_items.enums,
                context,
                &module_name,
                BASE_MODULE_CHAPTER_PREFIX,
            )?,
        ),
        (
            format!("{}-{}", module_name, TypeAlias::ITEMS_SUMMARY_FILENAME),
            generate_markdown_table_summary_for_top_level_subitems(
                &top_level_items.type_aliases,
                context,
                &module_name,
                BASE_MODULE_CHAPTER_PREFIX,
            )?,
        ),
        (
            format!("{}-{}", module_name, ImplAlias::ITEMS_SUMMARY_FILENAME),
            generate_markdown_table_summary_for_top_level_subitems(
                &top_level_items.impl_aliases,
                context,
                &module_name,
                BASE_MODULE_CHAPTER_PREFIX,
            )?,
        ),
        (
            format!("{}-{}", module_name, Trait::ITEMS_SUMMARY_FILENAME),
            generate_markdown_table_summary_for_top_level_subitems(
                &top_level_items.traits,
                context,
                &module_name,
                BASE_MODULE_CHAPTER_PREFIX,
            )?,
        ),
        (
            format!("{}-{}", module_name, Impl::ITEMS_SUMMARY_FILENAME),
            generate_markdown_table_summary_for_top_level_subitems(
                &top_level_items.impls,
                context,
                &module_name,
                BASE_MODULE_CHAPTER_PREFIX,
            )?,
        ),
        (
            format!("{}-{}", module_name, ExternType::ITEMS_SUMMARY_FILENAME),
            generate_markdown_table_summary_for_top_level_subitems(
                &top_level_items.extern_types,
                context,
                &module_name,
                BASE_MODULE_CHAPTER_PREFIX,
            )?,
        ),
        (
            format!("{}-{}", module_name, ExternFunction::ITEMS_SUMMARY_FILENAME),
            generate_markdown_table_summary_for_top_level_subitems(
                &top_level_items.extern_functions,
                context,
                &module_name,
                BASE_MODULE_CHAPTER_PREFIX,
            )?,
        ),
    ]
    .into_iter()
    .filter(|(_filename, content)| !content.is_empty())
    .collect::<Vec<_>>())
}

pub fn generate_markdown_list_summary_for_module_items<T: TopLevelMarkdownDocItem>(
    subitems: &[&T],
    mut nesting_level: usize,
    module_name: &String,
) -> Result<String> {
    let mut markdown = String::new();
    if !subitems.is_empty() {
        writeln!(
            &mut markdown,
            "{}- [{}](./{}-{})",
            "  ".repeat(nesting_level),
            T::HEADER,
            module_name,
            T::ITEMS_SUMMARY_FILENAME
        )?;
        nesting_level += 1;
        let items_with_relative_path = mark_duplicated_item_with_relative_path(subitems);
        for (item, relative_path) in items_with_relative_path {
            writeln!(
                &mut markdown,
                "  {}",
                item.generate_markdown_nested_list_item(relative_path, nesting_level)
            )?;
        }
    }
    Ok(markdown)
}

fn generate_top_level_docs_contents(
    items: &[&impl TopLevelMarkdownDocItem],
    context: &MarkdownGenerationContext,
) -> Result<Vec<(Filename, String)>> {
    items
        .iter()
        .map(|item| {
            let filename = item.filename();
            item.generate_markdown(context, BASE_HEADER_LEVEL, None)
                .map(|markdown| (filename, markdown))
        })
        .collect()
}
