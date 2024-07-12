use crate::docs_generation::markdown::traits::generate_markdown_list_for_top_level_subitems;
use crate::docs_generation::markdown::BASE_HEADER_LEVEL;
use crate::docs_generation::TopLevelItems;

pub(super) fn generate_summary_file_content(top_level_items: &TopLevelItems) -> String {
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

    markdown +=
        &generate_markdown_list_for_top_level_subitems(modules, "Modules", BASE_HEADER_LEVEL);
    markdown +=
        &generate_markdown_list_for_top_level_subitems(constants, "Constants", BASE_HEADER_LEVEL);
    markdown += &generate_markdown_list_for_top_level_subitems(
        free_functions,
        "Free functions",
        BASE_HEADER_LEVEL,
    );
    markdown +=
        &generate_markdown_list_for_top_level_subitems(structs, "Structs", BASE_HEADER_LEVEL);
    markdown += &generate_markdown_list_for_top_level_subitems(enums, "Enums", BASE_HEADER_LEVEL);
    markdown += &generate_markdown_list_for_top_level_subitems(
        type_aliases,
        "Type Aliases",
        BASE_HEADER_LEVEL,
    );
    markdown += &generate_markdown_list_for_top_level_subitems(
        impl_aliases,
        "Impl Aliases",
        BASE_HEADER_LEVEL,
    );
    markdown += &generate_markdown_list_for_top_level_subitems(traits, "Traits", BASE_HEADER_LEVEL);
    markdown += &generate_markdown_list_for_top_level_subitems(impls, "Impls", BASE_HEADER_LEVEL);
    markdown += &generate_markdown_list_for_top_level_subitems(
        extern_types,
        "Extern types",
        BASE_HEADER_LEVEL,
    );
    markdown += &generate_markdown_list_for_top_level_subitems(
        extern_functions,
        "Extern functions",
        BASE_HEADER_LEVEL,
    );

    markdown
}
