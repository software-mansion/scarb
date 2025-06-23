use crate::docs_generation::markdown::context::MarkdownGenerationContext;
use crate::docs_generation::markdown::traits::{
    MarkdownDocItem, TopLevelMarkdownDocItem,
    generate_markdown_table_summary_for_top_level_subitems,
};
use crate::docs_generation::markdown::{
    BASE_HEADER_LEVEL, BASE_MODULE_CHAPTER_PREFIX, Filename, SummaryIndexMap,
};
use crate::docs_generation::{DocItem, TopLevelItems};
use crate::types::module_type::Module;
use crate::types::other_types::{
    Constant, Enum, ExternFunction, ExternType, FreeFunction, Impl, ImplAlias, Struct, Trait,
    TypeAlias,
};
use anyhow::Result;
use itertools::chain;

macro_rules! module_summary {
    ($items:expr, $context:expr, $module_name:expr, $prefix:expr, [ $( $item_type:ty => $field:ident ),* ]) => {
        vec![
            $(
                (
                    format!("{}-{}", $module_name, <$item_type>::ITEMS_SUMMARY_FILENAME),
                    generate_markdown_table_summary_for_top_level_subitems(
                        &$items.$field,
                        $context,
                        &$module_name,
                        $prefix,
                    )?,
                )
            ),*
        ]
    };
}

pub fn generate_modules_summary_files(
    module: &Module,
    context: &MarkdownGenerationContext,
    summary_index_map: &SummaryIndexMap,
) -> Result<Vec<(String, String)>> {
    let mut top_level_items = TopLevelItems::default();
    let Module {
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

pub fn generate_foreign_crates_summary_files(
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

pub fn generate_summary_files_for_module_items(
    top_level_items: &TopLevelItems,
    module_name: String,
    context: &MarkdownGenerationContext,
) -> Result<Vec<(String, String)>> {
    Ok(module_summary!(
    top_level_items,
    context,
    module_name,
    BASE_MODULE_CHAPTER_PREFIX,
    [
        Module => modules,
        Constant => constants,
        FreeFunction => free_functions,
        Struct => structs,
        Enum => enums,
        TypeAlias => type_aliases,
        ImplAlias => impl_aliases,
        Trait => traits,
        Impl => impls,
        ExternType => extern_types,
        ExternFunction => extern_functions
    ])
    .into_iter()
    .filter(|(_filename, content)| !content.is_empty())
    .collect::<Vec<_>>())
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
