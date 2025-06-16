use crate::docs_generation::markdown::context::path_to_file_link;
use crate::docs_generation::markdown::traits::{
    TopLevelMarkdownDocItem, mark_duplicated_item_with_relative_path,
};
use crate::docs_generation::markdown::{SummaryIndexMap, SummaryListItem};
use crate::docs_generation::{DocItem, TopLevelItems};
use crate::types::groups::Group;
use crate::types::module_type::Module;

macro_rules! insert_multiple_summaries {
    ($summary_index_map:expr, $items:expr, $nesting_level:expr, $path:expr, [ $( $field:ident ),* ]) => {
        $(
            $summary_index_map.extend(generate_markdown_list_summary_for_module_items(
                &$items.$field,
                $nesting_level,
                $path,
            ));
        )*
    };
}

pub fn generate_module_summary_content(
    module: &Module,
    mut nesting_level: usize,
    summary_index_map: &mut SummaryIndexMap,
) {
    summary_index_map.insert(
        path_to_file_link(module.full_path()),
        SummaryListItem::new(module.item_data.name.clone(), nesting_level),
    );

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

    nesting_level += 1;
    if !top_level_items.modules.is_empty() {
        summary_index_map.insert(
            format!(
                "./{}-{}",
                module.markdown_formatted_path(),
                Module::ITEMS_SUMMARY_FILENAME
            ),
            SummaryListItem::new(Module::HEADER.to_string(), nesting_level),
        );
        nesting_level += 1;

        for submodule in module.submodules.iter() {
            generate_module_summary_content(submodule, nesting_level, summary_index_map);
        }
        nesting_level -= 1;
    }

    insert_multiple_summaries!(
        summary_index_map,
        top_level_items,
        nesting_level,
        &module.markdown_formatted_path(),
        [
            constants,
            free_functions,
            structs,
            enums,
            type_aliases,
            impl_aliases,
            traits,
            impls,
            extern_types,
            extern_functions
        ]
    );
}

pub fn generate_foreign_crates_summary_content(
    foreign_modules: &Vec<Module>,
    summary_index_map: &mut SummaryIndexMap,
) {
    for module in foreign_modules {
        generate_module_summary_content(module, 0, summary_index_map);
    }
}

pub fn generate_global_groups_summary_content(
    groups: &[Group],
    summary_index_map: &mut SummaryIndexMap,
) {
    if !groups.is_empty() {
        summary_index_map.insert(
            "".to_string(),
            SummaryListItem::new("Groups".to_string(), 0),
        );

        let mut nesting_level = 2;

        for group in groups.iter() {
            let mut top_level_items = TopLevelItems::default();
            let Group {
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

            summary_index_map.insert(
                group.filename(),
                SummaryListItem::new(group.name.to_string(), 1),
            );
            let markdown_formatted_path = group.get_name_normalized();

            if !top_level_items.modules.is_empty() {
                summary_index_map.insert(
                    format!(
                        "./{}-{}",
                        markdown_formatted_path,
                        Module::ITEMS_SUMMARY_FILENAME,
                    ),
                    SummaryListItem::new(Module::HEADER.to_string(), nesting_level),
                );
                nesting_level += 1;
                for submodule in group.submodules.iter() {
                    generate_module_summary_content(submodule, nesting_level, summary_index_map);
                }
                nesting_level -= 1;
            };
            insert_multiple_summaries!(
                summary_index_map,
                top_level_items,
                nesting_level,
                &markdown_formatted_path,
                [
                    constants,
                    free_functions,
                    structs,
                    enums,
                    type_aliases,
                    impl_aliases,
                    traits,
                    impls,
                    extern_types,
                    extern_functions
                ]
            );
        }
    }
}

fn generate_markdown_list_summary_for_module_items<T: TopLevelMarkdownDocItem>(
    subitems: &[&T],
    mut nesting_level: usize,
    module_name: &String,
) -> Vec<(String, SummaryListItem)> {
    let mut summary_items: Vec<(String, SummaryListItem)> = vec![];
    if !subitems.is_empty() {
        summary_items.push((
            format!("./{}-{}", module_name, T::ITEMS_SUMMARY_FILENAME,),
            SummaryListItem::new(T::HEADER.to_string(), nesting_level),
        ));
        nesting_level += 1;
        let items_with_relative_path = mark_duplicated_item_with_relative_path(subitems);
        for (item, relative_path) in items_with_relative_path {
            let (file_item, name_item) = item.get_markdown_nested_list_item(relative_path);
            summary_items.push((file_item, SummaryListItem::new(name_item, nesting_level)));
        }
    }
    summary_items
}
