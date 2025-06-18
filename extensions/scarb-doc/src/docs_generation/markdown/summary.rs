pub(crate) mod content;

use super::traits::{MarkdownDocItem, generate_markdown_table_summary_for_top_level_subitems};
use crate::docs_generation::markdown::context::MarkdownGenerationContext;
use crate::docs_generation::markdown::groups::generate_global_groups_summary_files;
use crate::docs_generation::markdown::summary::content::{
    generate_foreign_crates_summary_content, generate_global_groups_summary_content,
    generate_module_summary_content,
};
use crate::docs_generation::markdown::traits::TopLevelMarkdownDocItem;
use crate::docs_generation::markdown::{
    BASE_HEADER_LEVEL, BASE_MODULE_CHAPTER_PREFIX, Filename, SummaryIndexMap,
};
use crate::docs_generation::{DocItem, TopLevelItems};
use crate::types::crate_type::Crate;
use crate::types::module_type::Module;
use crate::types::other_types::{
    Constant, Enum, ExternFunction, ExternType, FreeFunction, Impl, ImplAlias, Struct, Trait,
    TypeAlias,
};
use anyhow::Result;
use itertools::chain;

pub fn generate_summary_file_content(
    crate_: &Crate,
) -> Result<(SummaryIndexMap, Vec<(String, String)>)> {
    let mut summary_index_map = SummaryIndexMap::new();
    let context = MarkdownGenerationContext::from_crate(crate_);

    generate_module_summary_content(&crate_.root_module, 0, &mut summary_index_map);
    generate_foreign_crates_summary_content(&crate_.foreign_crates, &mut summary_index_map);
    generate_global_groups_summary_content(&crate_.groups, &mut summary_index_map);

    let mut summary_files = vec![(
        crate_.root_module.filename(),
        crate_.root_module.generate_markdown(
            &context,
            BASE_HEADER_LEVEL,
            None,
            &summary_index_map,
        )?,
    )];

    let module_item_summaries =
        &generate_modules_summary_files(&crate_.root_module, &context, &summary_index_map)?;
    summary_files.extend(module_item_summaries.to_owned());

    let foreign_modules_files = generate_foreign_crates_summary_files(
        &crate_.foreign_crates,
        &context,
        &summary_index_map,
    )?;

    summary_files.extend(foreign_modules_files);

    let groups_files =
        generate_global_groups_summary_files(&crate_.groups, &context, &summary_index_map)?;
    summary_files.extend(groups_files.to_owned());
    Ok((summary_index_map, summary_files))
}

fn generate_foreign_crates_summary_files(
    foreign_modules: &Vec<Module>,
    context: &MarkdownGenerationContext,
    summary_index_map: &SummaryIndexMap,
) -> Result<Vec<(String, String)>> {
    let mut summary_files = vec![];

    for module in foreign_modules {
        summary_files.extend(vec![(
            module.filename(),
            module.generate_markdown(context, BASE_HEADER_LEVEL, None, summary_index_map)?,
        )]);
        let module_item_summaries =
            &generate_modules_summary_files(module, context, summary_index_map)?;
        summary_files.extend(module_item_summaries.to_owned());
    }
    Ok(summary_files)
}

pub fn generate_modules_summary_files(
    module: &Module,
    context: &MarkdownGenerationContext,
    summary_index_map: &SummaryIndexMap,
) -> Result<Vec<(String, String)>> {
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
        generate_doc_files_for_module_items(&top_level_items, context, summary_index_map)?
            .to_owned(),
    );

    if !top_level_items.modules.is_empty() {
        for submodule in module.submodules.iter() {
            let sub_summaries =
                &generate_modules_summary_files(submodule, context, summary_index_map)?;
            doc_files.extend::<Vec<(String, String)>>(sub_summaries.to_owned());
        }
    }
    Ok(doc_files)
}

pub fn generate_doc_files_for_module_items(
    top_level_items: &TopLevelItems,
    context: &MarkdownGenerationContext,
    summary_index_map: &SummaryIndexMap,
) -> Result<Vec<(String, String)>> {
    Ok(chain!(
        generate_top_level_docs_contents(&top_level_items.modules, context, summary_index_map)?,
        generate_top_level_docs_contents(&top_level_items.constants, context, summary_index_map)?,
        generate_top_level_docs_contents(
            &top_level_items.free_functions,
            context,
            summary_index_map
        )?,
        generate_top_level_docs_contents(&top_level_items.structs, context, summary_index_map)?,
        generate_top_level_docs_contents(&top_level_items.enums, context, summary_index_map)?,
        generate_top_level_docs_contents(
            &top_level_items.type_aliases,
            context,
            summary_index_map
        )?,
        generate_top_level_docs_contents(
            &top_level_items.impl_aliases,
            context,
            summary_index_map,
        )?,
        generate_top_level_docs_contents(&top_level_items.traits, context, summary_index_map,)?,
        generate_top_level_docs_contents(&top_level_items.impls, context, summary_index_map,)?,
        generate_top_level_docs_contents(
            &top_level_items.extern_types,
            context,
            summary_index_map,
        )?,
        generate_top_level_docs_contents(
            &top_level_items.extern_functions,
            context,
            summary_index_map,
        )?,
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

fn generate_top_level_docs_contents(
    items: &[&impl TopLevelMarkdownDocItem],
    context: &MarkdownGenerationContext,
    summary_index_map: &SummaryIndexMap,
) -> Result<Vec<(Filename, String)>> {
    items
        .iter()
        .map(|item| {
            item.generate_markdown(context, BASE_HEADER_LEVEL, None, summary_index_map)
                .map(|markdown| (item.filename(), markdown))
        })
        .collect()
}
