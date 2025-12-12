use super::context::MarkdownGenerationContext;
use crate::docs_generation::markdown::{
    BASE_MODULE_CHAPTER_PREFIX, GROUP_CHAPTER_PREFIX, SHORT_DOCUMENTATION_AVOID_PREFIXES,
    SHORT_DOCUMENTATION_LEN, SummaryIndexMap,
};
use crate::docs_generation::{DocItem, PrimitiveDocItem, SubPathDocItem, TopLevelDocItem};
use crate::types::groups::Group;
use crate::types::item_data::{ItemData, SubItemData};
use crate::types::module_type::{Module, ModulePubUses};
use crate::types::other_types::{
    Constant, Enum, ExternFunction, ExternType, FreeFunction, Impl, ImplAlias, ImplConstant,
    ImplFunction, ImplType, MacroDeclaration, Member, Struct, Trait, TraitConstant, TraitFunction,
    TraitType, TypeAlias, Variant,
};
use anyhow::Result;
use cairo_lang_doc::parser::{CommentLinkToken, DocumentationCommentToken};
use itertools::Itertools;
use std::collections::HashMap;
use std::fmt::Write;
use std::option::Option;
use std::path::Path;

const RE_EXPORTS_CHAPTER: &str = "\n\n---\n \n# Re-exports: \n";
const GROUPS_CHAPTER: &str = "\n\n---\n \n# Groups: \n";

pub trait TopLevelMarkdownDocItem: MarkdownDocItem + TopLevelDocItem {
    const ITEMS_SUMMARY_FILENAME: &'static str;

    fn filename(&self, files_extension: &str) -> String {
        format!("{}{files_extension}", self.markdown_formatted_path())
    }

    fn md_ref_formatted(&self, relative_path: Option<String>, files_extension: &str) -> String {
        let (path, filename) = self.md_ref(relative_path, files_extension);
        format!("[{path}](./{filename})")
    }

    fn md_ref(&self, relative_path: Option<String>, files_extension: &str) -> (String, String) {
        match relative_path {
            Some(path) => (path, self.filename(files_extension)),
            None => (self.name().to_string(), self.filename(files_extension)),
        }
    }

    fn get_markdown_nested_list_item(
        &self,
        relative_path: Option<String>,
        files_extension: &str,
    ) -> (String, String) {
        let (path, filename) = self.md_ref(relative_path, files_extension);
        (format!("./{filename}"), path)
    }
}

macro_rules! impl_top_level_markdown_doc_item {
    ($t:ty, $filename:expr) => {
        impl TopLevelMarkdownDocItem for $t {
            const ITEMS_SUMMARY_FILENAME: &'static str = $filename;
        }
    };
}

impl_top_level_markdown_doc_item!(Constant<'_>, "constants");
impl_top_level_markdown_doc_item!(Enum<'_>, "enums");
impl_top_level_markdown_doc_item!(ExternFunction<'_>, "extern_functions");
impl_top_level_markdown_doc_item!(ExternType<'_>, "extern_types");
impl_top_level_markdown_doc_item!(FreeFunction<'_>, "free_functions");
impl_top_level_markdown_doc_item!(Impl<'_>, "impls");
impl_top_level_markdown_doc_item!(ImplAlias<'_>, "impl_aliases");
impl_top_level_markdown_doc_item!(Module<'_>, "modules");
impl_top_level_markdown_doc_item!(Struct<'_>, "structs");
impl_top_level_markdown_doc_item!(Trait<'_>, "traits");
impl_top_level_markdown_doc_item!(TypeAlias<'_>, "type_aliases");
impl_top_level_markdown_doc_item!(MacroDeclaration<'_>, "macro_declarations");

macro_rules! impl_markdown_doc_item {
    ($ty:ty) => {
        impl MarkdownDocItem for $ty {
            fn generate_markdown(
                &self,
                context: &MarkdownGenerationContext,
                header_level: usize,
                item_suffix: Option<usize>,
                summary_index_map: &SummaryIndexMap,
            ) -> Result<String> {
                let mut markdown = String::new();

                let header =
                    context.get_header_primitive(header_level, self.name(), self.full_path());
                writeln!(&mut markdown, "{}\n", header)?;

                if let Some(doc) = self.get_documentation(context) {
                    writeln!(&mut markdown, "{doc}\n")?;
                }

                let full_path = self.get_full_path(item_suffix, context.files_extension);
                if let Some(fully_qualified_path) = context.get_fully_qualified_path(full_path) {
                    writeln!(&mut markdown, "{}\n", fully_qualified_path)?;
                }

                if let Some(group_name) = self.group_name() {
                    writeln!(&mut markdown, "{}", context.get_group(group_name))?
                }

                if let Some(sig) = &self.signature()
                    && !sig.is_empty()
                {
                    let signature =
                        context.get_signature(sig, self.doc_location_links(), summary_index_map);
                    writeln!(&mut markdown, "{}", signature)?;
                }
                Ok(markdown)
            }

            fn get_full_path(&self, item_suffix: Option<usize>, files_extension: &str) -> String {
                get_full_subitem_path(self, item_suffix, files_extension)
            }
        }
    };
}

impl_markdown_doc_item!(Member<'_>);
impl_markdown_doc_item!(ImplFunction<'_>);
impl_markdown_doc_item!(ImplType<'_>);
impl_markdown_doc_item!(TraitFunction<'_>);
impl_markdown_doc_item!(Variant<'_>);
impl_markdown_doc_item!(ImplConstant<'_>);
impl_markdown_doc_item!(TraitConstant<'_>);
impl_markdown_doc_item!(TraitType<'_>);

pub trait MarkdownDocItem: DocItem {
    fn generate_markdown(
        &self,
        context: &MarkdownGenerationContext,
        header_level: usize,
        item_suffix: Option<usize>,
        summary_index_map: &SummaryIndexMap,
    ) -> Result<String>;

    fn get_short_documentation(&self, context: &MarkdownGenerationContext) -> String {
        let mut short_doc_buff = String::new();

        if let Some(tokens) = self.doc() {
            for token in tokens.iter() {
                let mut text_formatted = match token {
                    DocumentationCommentToken::Content(content) => {
                        if SHORT_DOCUMENTATION_AVOID_PREFIXES
                            .iter()
                            .any(|prefix| content.starts_with(prefix))
                        {
                            return format!("{}...", short_doc_buff.trim());
                        } else if !content.eq("\n") {
                            content.replace("\n", " ")
                        } else {
                            continue;
                        }
                    }
                    DocumentationCommentToken::Link(link) => {
                        self.format_link_to_path(link, context)
                    }
                };
                if !text_formatted.ends_with(' ') {
                    text_formatted.push(' ');
                }

                if short_doc_buff.len() + text_formatted.len() > SHORT_DOCUMENTATION_LEN {
                    return format!("{}...", short_doc_buff.trim());
                } else {
                    short_doc_buff.push_str(&text_formatted);
                }
            }

            let short_doc = short_doc_buff.trim().to_string();
            return if short_doc.is_empty() {
                "—".to_string()
            } else {
                short_doc
            };
        }
        "—".to_string()
    }

    fn get_documentation(&self, context: &MarkdownGenerationContext) -> Option<String> {
        self.doc().as_ref().map(|doc_tokens| {
            doc_tokens
                .iter()
                .map(|doc_token| match doc_token {
                    DocumentationCommentToken::Content(content) => content.clone(),
                    DocumentationCommentToken::Link(link) => {
                        self.format_link_to_path(link, context)
                    }
                })
                .join("")
        })
    }

    fn format_link_to_path(
        &self,
        link: &CommentLinkToken,
        context: &MarkdownGenerationContext,
    ) -> String {
        if let Some(file_path) = context.resolve_markdown_file_path_from_link(link) {
            format!("[{}]({file_path})", link.label.clone(),)
        } else {
            link.label.clone()
        }
    }

    fn get_full_path(&self, _item_suffix: Option<usize>, files_extension: &str) -> String {
        get_linked_path(self.full_path(), files_extension)
    }

    fn get_source_code_link(&self, context: &MarkdownGenerationContext) -> Option<String> {
        if let Some(base_ur) = context.remote_base_url.clone()
            && let Some(file_path) = self.file_path()
        {
            let full_path = Path::new(file_path);

            let mut root_ws = context.workspace_root.clone();
            root_ws.pop(); // preserve project root dir

            return match full_path.strip_prefix(root_ws) {
                Ok(relative_path) => {
                    let relative_path_str = relative_path.to_str().unwrap_or("");
                    Some(format!(
                        "<a href='{base_ur}{relative_path_str}' target='blank'> [source code] </a>"
                    ))
                }
                Err(_) => None,
            };
        }
        None
    }
}

impl<T> MarkdownDocItem for T
where
    T: PrimitiveDocItem,
{
    fn generate_markdown(
        &self,
        context: &MarkdownGenerationContext,
        header_level: usize,
        _item_suffix: Option<usize>,
        summary_index_map: &SummaryIndexMap,
    ) -> Result<String> {
        generate_markdown_from_item_data(self, context, header_level, None, summary_index_map)
    }
}

impl<'db> MarkdownDocItem for Enum<'db> {
    fn generate_markdown(
        &self,
        context: &MarkdownGenerationContext,
        header_level: usize,
        _item_suffix: Option<usize>,
        summary_index_map: &SummaryIndexMap,
    ) -> Result<String> {
        let mut markdown =
            generate_markdown_from_item_data(self, context, header_level, None, summary_index_map)?;
        let mut suffix_calculator = ItemSuffixCalculator::new(self.name());
        markdown += &generate_markdown_for_subitems(
            &self.variants,
            context,
            header_level,
            &mut suffix_calculator,
            summary_index_map,
        )?;

        Ok(markdown)
    }
}

impl<'db> MarkdownDocItem for Impl<'db> {
    fn generate_markdown(
        &self,
        context: &MarkdownGenerationContext,
        header_level: usize,
        _item_suffix: Option<usize>,
        summary_index_map: &SummaryIndexMap,
    ) -> Result<String> {
        let mut markdown =
            generate_markdown_from_item_data(self, context, header_level, None, summary_index_map)?;
        let mut suffix_calculator = ItemSuffixCalculator::new(self.name());

        markdown += &generate_markdown_for_subitems(
            &self.impl_constants,
            context,
            header_level,
            &mut suffix_calculator,
            summary_index_map,
        )?;

        markdown += &generate_markdown_for_subitems(
            &self.impl_functions,
            context,
            header_level,
            &mut suffix_calculator,
            summary_index_map,
        )?;

        markdown += &generate_markdown_for_subitems(
            &self.impl_types,
            context,
            header_level,
            &mut suffix_calculator,
            summary_index_map,
        )?;

        Ok(markdown)
    }
}

fn generate_pub_use_item_markdown(
    module_pubuses: &ModulePubUses,
    context: &MarkdownGenerationContext,
) -> String {
    let mut buff: String = String::new();

    buff += &generate_markdown_table_summary_for_reexported_subitems(
        &module_pubuses.use_constants.iter().collect_vec(),
        context,
    )
    .unwrap_or("".to_string());

    buff += &generate_markdown_table_summary_for_reexported_subitems(
        &module_pubuses.use_free_functions.iter().collect_vec(),
        context,
    )
    .unwrap_or("".to_string());

    buff += &generate_markdown_table_summary_for_reexported_subitems(
        &module_pubuses.use_structs.iter().collect_vec(),
        context,
    )
    .unwrap_or("".to_string());

    buff += &generate_markdown_table_summary_for_reexported_subitems(
        &module_pubuses.use_enums.iter().collect_vec(),
        context,
    )
    .unwrap_or("".to_string());

    buff += &generate_markdown_table_summary_for_reexported_subitems(
        &module_pubuses.use_module_type_aliases.iter().collect_vec(),
        context,
    )
    .unwrap_or("".to_string());

    buff += &generate_markdown_table_summary_for_reexported_subitems(
        &module_pubuses.use_impl_aliases.iter().collect_vec(),
        context,
    )
    .unwrap_or("".to_string());

    buff += &generate_markdown_table_summary_for_reexported_subitems(
        &module_pubuses.use_traits.iter().collect_vec(),
        context,
    )
    .unwrap_or("".to_string());

    buff += &generate_markdown_table_summary_for_reexported_subitems(
        &module_pubuses.use_impl_defs.iter().collect_vec(),
        context,
    )
    .unwrap_or("".to_string());

    buff += &generate_markdown_table_summary_for_reexported_subitems(
        &module_pubuses.use_extern_types.iter().collect_vec(),
        context,
    )
    .unwrap_or("".to_string());

    buff += &generate_markdown_table_summary_for_reexported_subitems(
        &module_pubuses.use_extern_functions.iter().collect_vec(),
        context,
    )
    .unwrap_or("".to_string());

    buff += &generate_markdown_table_summary_for_reexported_subitems(
        &module_pubuses.use_submodules.iter().collect_vec(),
        context,
    )
    .unwrap_or("".to_string());

    buff += &generate_markdown_table_summary_for_reexported_subitems(
        &module_pubuses.use_macro_declarations.iter().collect_vec(),
        context,
    )
    .unwrap_or("".to_string());

    if !buff.is_empty() {
        return format!("{RE_EXPORTS_CHAPTER}{buff}");
    }
    buff
}

impl<'db> MarkdownDocItem for Module<'db> {
    fn generate_markdown(
        &self,
        context: &MarkdownGenerationContext,
        header_level: usize,
        _item_suffix: Option<usize>,
        summary_index_map: &SummaryIndexMap,
    ) -> Result<String> {
        let mut markdown =
            generate_markdown_from_item_data(self, context, header_level, None, summary_index_map)?;

        markdown += &generate_markdown_table_summary_for_top_level_subitems(
            &self.submodules.iter().collect_vec(),
            context,
            &self.markdown_formatted_path(),
            BASE_MODULE_CHAPTER_PREFIX,
        )?;
        markdown += &generate_markdown_table_summary_for_top_level_subitems(
            &self.constants.iter().collect_vec(),
            context,
            &self.markdown_formatted_path(),
            BASE_MODULE_CHAPTER_PREFIX,
        )?;
        markdown += &generate_markdown_table_summary_for_top_level_subitems(
            &self.free_functions.iter().collect_vec(),
            context,
            &self.markdown_formatted_path(),
            BASE_MODULE_CHAPTER_PREFIX,
        )?;
        markdown += &generate_markdown_table_summary_for_top_level_subitems(
            &self.structs.iter().collect_vec(),
            context,
            &self.markdown_formatted_path(),
            BASE_MODULE_CHAPTER_PREFIX,
        )?;
        markdown += &generate_markdown_table_summary_for_top_level_subitems(
            &self.enums.iter().collect_vec(),
            context,
            &self.markdown_formatted_path(),
            BASE_MODULE_CHAPTER_PREFIX,
        )?;
        markdown += &generate_markdown_table_summary_for_top_level_subitems(
            &self.type_aliases.iter().collect_vec(),
            context,
            &self.markdown_formatted_path(),
            BASE_MODULE_CHAPTER_PREFIX,
        )?;
        markdown += &generate_markdown_table_summary_for_top_level_subitems(
            &self.impl_aliases.iter().collect_vec(),
            context,
            &self.markdown_formatted_path(),
            BASE_MODULE_CHAPTER_PREFIX,
        )?;
        markdown += &generate_markdown_table_summary_for_top_level_subitems(
            &self.traits.iter().collect_vec(),
            context,
            &self.markdown_formatted_path(),
            BASE_MODULE_CHAPTER_PREFIX,
        )?;
        markdown += &generate_markdown_table_summary_for_top_level_subitems(
            &self.impls.iter().collect_vec(),
            context,
            &self.markdown_formatted_path(),
            BASE_MODULE_CHAPTER_PREFIX,
        )?;
        markdown += &generate_markdown_table_summary_for_top_level_subitems(
            &self.extern_types.iter().collect_vec(),
            context,
            &self.markdown_formatted_path(),
            BASE_MODULE_CHAPTER_PREFIX,
        )?;
        markdown += &generate_markdown_table_summary_for_top_level_subitems(
            &self.extern_functions.iter().collect_vec(),
            context,
            &self.markdown_formatted_path(),
            BASE_MODULE_CHAPTER_PREFIX,
        )?;
        markdown += &generate_markdown_table_summary_for_top_level_subitems(
            &self.macro_declarations.iter().collect_vec(),
            context,
            &self.markdown_formatted_path(),
            BASE_MODULE_CHAPTER_PREFIX,
        )?;
        markdown += &generate_pub_use_item_markdown(&self.pub_uses, context);

        if !self.groups.is_empty() {
            markdown += GROUPS_CHAPTER;
            markdown += &generate_markdown_table_summary_for_top_level_groups_items(
                &self.groups.iter().collect_vec(),
                context,
            )?;
        }
        Ok(markdown)
    }
}

impl<'db> MarkdownDocItem for Struct<'db> {
    fn generate_markdown(
        &self,
        context: &MarkdownGenerationContext,
        header_level: usize,
        _item_suffix: Option<usize>,
        summary_index_map: &SummaryIndexMap,
    ) -> Result<String> {
        let mut markdown =
            generate_markdown_from_item_data(self, context, header_level, None, summary_index_map)?;

        let mut suffix_calculator = ItemSuffixCalculator::new(self.name());
        markdown += &generate_markdown_for_subitems(
            &self.members,
            context,
            header_level,
            &mut suffix_calculator,
            summary_index_map,
        )?;

        Ok(markdown)
    }
}

impl<'db> MarkdownDocItem for Trait<'db> {
    fn generate_markdown(
        &self,
        context: &MarkdownGenerationContext,
        header_level: usize,
        _item_suffix: Option<usize>,
        summary_index_map: &SummaryIndexMap,
    ) -> Result<String> {
        let mut markdown =
            generate_markdown_from_item_data(self, context, header_level, None, summary_index_map)?;
        let mut suffix_calculator = ItemSuffixCalculator::new(self.name());

        markdown += &generate_markdown_for_subitems(
            &self.trait_constants,
            context,
            header_level,
            &mut suffix_calculator,
            summary_index_map,
        )?;
        markdown += &generate_markdown_for_subitems(
            &self.trait_functions,
            context,
            header_level,
            &mut suffix_calculator,
            summary_index_map,
        )?;
        markdown += &generate_markdown_for_subitems(
            &self.trait_types,
            context,
            header_level,
            &mut suffix_calculator,
            summary_index_map,
        )?;
        Ok(markdown)
    }
}

struct ItemSuffixCalculator<'a> {
    occurrences: HashMap<String, usize>,
    parent_name: &'a str,
}

impl<'a> ItemSuffixCalculator<'a> {
    pub fn new(parent_name: &'a str) -> Self {
        Self {
            occurrences: HashMap::new(),
            parent_name,
        }
    }

    pub fn get(&mut self, item: &str) -> Option<usize> {
        let lowercase_item = item.to_lowercase();
        let mut count = self.occurrences.get(&lowercase_item).copied().unwrap_or(0);
        if self.parent_name.to_lowercase() == lowercase_item {
            count += 1;
        }
        let result = if count == 0 { None } else { Some(count) };
        *self.occurrences.entry(lowercase_item).or_insert(0) += 1;
        result
    }
}

/// Takes items, and appends for each of them a path, that was trimmed based on the common prefix of all the items,
/// that share the same name.
pub fn mark_duplicated_item_with_relative_path<'a, T: TopLevelMarkdownDocItem + 'a>(
    items: &'a [&'a T],
) -> Vec<(&'a &'a T, Option<String>)> {
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

pub fn generate_markdown_table_summary_for_top_level_subitems<T: TopLevelMarkdownDocItem>(
    subitems: &[&T],
    context: &MarkdownGenerationContext,
    module_name: &String,
    prefix: &str,
) -> Result<String> {
    let mut markdown = String::new();

    if !subitems.is_empty() {
        let linked = format!(
            "[{}](./{}-{}{})",
            T::HEADER,
            module_name,
            T::ITEMS_SUMMARY_FILENAME,
            context.files_extension,
        );

        writeln!(&mut markdown, "\n{prefix} {linked}\n\n| | |\n|:---|:---|",)?;

        let items_with_relative_path = mark_duplicated_item_with_relative_path(subitems);
        for (item, relative_path) in items_with_relative_path {
            let item_doc = item.get_short_documentation(context);
            writeln!(
                &mut markdown,
                "| {} | {} |",
                item.md_ref_formatted(relative_path, context.files_extension),
                item_doc,
            )?;
        }
    }

    Ok(markdown)
}

pub fn generate_markdown_table_summary_for_top_level_groups_items(
    groups: &[&Group],
    context: &MarkdownGenerationContext,
) -> Result<String> {
    let mut markdown = String::new();

    if !groups.is_empty() {
        for group in groups {
            markdown += &format!(
                "\n## [{}]({})\n",
                group.name,
                group.filename(context.files_extension),
            );

            let fake_module_name = group.get_name_normalized();
            markdown += &generate_markdown_table_summary_for_top_level_subitems(
                &group.submodules.iter().collect_vec(),
                context,
                &fake_module_name,
                GROUP_CHAPTER_PREFIX,
            )?;
            markdown += &generate_markdown_table_summary_for_top_level_subitems(
                &group.constants.iter().collect_vec(),
                context,
                &fake_module_name,
                GROUP_CHAPTER_PREFIX,
            )?;
            markdown += &generate_markdown_table_summary_for_top_level_subitems(
                &group.free_functions.iter().collect_vec(),
                context,
                &fake_module_name,
                GROUP_CHAPTER_PREFIX,
            )?;
            markdown += &generate_markdown_table_summary_for_top_level_subitems(
                &group.structs.iter().collect_vec(),
                context,
                &fake_module_name,
                GROUP_CHAPTER_PREFIX,
            )?;
            markdown += &generate_markdown_table_summary_for_top_level_subitems(
                &group.enums.iter().collect_vec(),
                context,
                &fake_module_name,
                GROUP_CHAPTER_PREFIX,
            )?;
            markdown += &generate_markdown_table_summary_for_top_level_subitems(
                &group.type_aliases.iter().collect_vec(),
                context,
                &fake_module_name,
                GROUP_CHAPTER_PREFIX,
            )?;
            markdown += &generate_markdown_table_summary_for_top_level_subitems(
                &group.impl_aliases.iter().collect_vec(),
                context,
                &fake_module_name,
                GROUP_CHAPTER_PREFIX,
            )?;
            markdown += &generate_markdown_table_summary_for_top_level_subitems(
                &group.traits.iter().collect_vec(),
                context,
                &fake_module_name,
                GROUP_CHAPTER_PREFIX,
            )?;
            markdown += &generate_markdown_table_summary_for_top_level_subitems(
                &group.impls.iter().collect_vec(),
                context,
                &fake_module_name,
                GROUP_CHAPTER_PREFIX,
            )?;
            markdown += &generate_markdown_table_summary_for_top_level_subitems(
                &group.extern_types.iter().collect_vec(),
                context,
                &fake_module_name,
                GROUP_CHAPTER_PREFIX,
            )?;
            markdown += &generate_markdown_table_summary_for_top_level_subitems(
                &group.extern_functions.iter().collect_vec(),
                context,
                &fake_module_name,
                GROUP_CHAPTER_PREFIX,
            )?;
        }
    }
    Ok(markdown)
}

pub fn generate_markdown_table_summary_for_reexported_subitems<T: TopLevelMarkdownDocItem>(
    subitems: &[&T],
    context: &MarkdownGenerationContext,
) -> Result<String> {
    let mut markdown = String::new();
    if !subitems.is_empty() {
        writeln!(
            &mut markdown,
            "\n - ### {}\n\n| | |\n|:---|:---|",
            T::HEADER,
        )?;
        let items_with_relative_path = mark_duplicated_item_with_relative_path(subitems);
        for (item, relative_path) in items_with_relative_path {
            let item_doc = item.get_short_documentation(context);
            writeln!(
                &mut markdown,
                "| {} | {} |",
                item.md_ref_formatted(relative_path, context.files_extension),
                item_doc,
            )?;
        }
        writeln!(&mut markdown, "\n<br>\n")?;
    }
    Ok(markdown)
}

fn generate_markdown_for_subitems<T: MarkdownDocItem + SubPathDocItem>(
    subitems: &[T],
    context: &MarkdownGenerationContext,
    header_level: usize,
    suffix_calculator: &mut ItemSuffixCalculator,
    summary_index_map: &SummaryIndexMap,
) -> Result<String> {
    let mut markdown = String::new();

    if !subitems.is_empty() {
        let header = str::repeat("#", header_level + 1);

        writeln!(&mut markdown, "{header} {}\n", T::HEADER)?;

        for item in subitems.iter() {
            let postfix = suffix_calculator.get(item.name());
            writeln!(
                &mut markdown,
                "{}",
                item.generate_markdown(context, header_level + 2, postfix, summary_index_map)?
            )?;
        }
    }

    Ok(markdown)
}

fn generate_markdown_from_item_data(
    doc_item: &impl MarkdownDocItem,
    context: &MarkdownGenerationContext,
    header_level: usize,
    item_suffix: Option<usize>,
    summary_index_map: &SummaryIndexMap,
) -> Result<String> {
    let mut markdown = String::new();

    let header = context.get_header(header_level, doc_item.name(), doc_item.full_path());
    writeln!(&mut markdown, "{}\n", header)?;

    if let Some(source_code_link) = doc_item.get_source_code_link(context) {
        writeln!(&mut markdown, "{source_code_link}\n")?;
    }

    if let Some(doc) = doc_item.get_documentation(context) {
        writeln!(&mut markdown, "{doc}\n")?;
    }

    if let Some(fully_qualified_path) = context
        .get_fully_qualified_path(doc_item.get_full_path(item_suffix, context.files_extension))
    {
        writeln!(&mut markdown, "{fully_qualified_path}\n")?;
    }

    if let Some(group_name) = doc_item.group_name() {
        writeln!(&mut markdown, "{}", context.get_group(group_name))?
    }

    if let Some(sig) = &doc_item.signature()
        && !sig.is_empty()
    {
        let signature =
            context.get_signature(sig, doc_item.doc_location_links(), summary_index_map);
        writeln!(&mut markdown, "{signature}")?;
    }

    Ok(markdown)
}

fn get_linked_path(full_path: &str, files_extension: &str) -> String {
    let path_items = full_path.split("::").collect::<Vec<_>>();
    let mut result: Vec<String> = Vec::new();
    let mut current_path = String::new();
    for element in path_items {
        if !current_path.is_empty() {
            current_path.push('-');
        }
        current_path.push_str(element);
        let formatted = format!("[{element}](./{}{files_extension})", &current_path,);
        result.push(formatted);
    }
    result.join("::")
}

/// Formats markdown path to a relevant chapter within the item parent page.
/// Differs from parent path by appended suffix that consist of child item name
/// and number of previous name occurrences within the parent page chapters.       
fn get_full_subitem_path<T: MarkdownDocItem + SubPathDocItem>(
    item: &T,
    item_suffix: Option<usize>,
    files_extension: &str,
) -> String {
    if let Some((parent_path, item_path)) = item.full_path().rsplit_once("::") {
        let last_path = format!(
            "{}{files_extension}#{}{}",
            parent_path.replace("::", "-"),
            item_path.to_lowercase(),
            if let Some(item_suffix) = item_suffix {
                format!("-{item_suffix}")
            } else {
                "".to_string()
            }
        );
        format!(
            "{}::[{}](./{})",
            get_linked_path(parent_path, files_extension),
            &item_path,
            last_path
        )
    } else {
        get_linked_path(item.full_path(), files_extension)
    }
}

pub trait WithPath {
    fn name(&self) -> &str;
    fn full_path(&self) -> String;
    fn parent_full_path(&self) -> Option<String>;
}

pub trait WithItemData {
    fn item_data(&self) -> &ItemData<'_>;
}

impl<T: WithItemData> WithPath for T {
    fn name(&self) -> &str {
        self.item_data().name.as_str()
    }

    fn full_path(&self) -> String {
        self.item_data().full_path.clone()
    }

    fn parent_full_path(&self) -> Option<String> {
        self.item_data().parent_full_path.clone()
    }
}

impl<'db> WithItemData for ItemData<'db> {
    fn item_data(&self) -> &ItemData<'_> {
        self
    }
}

// Allow SubItemData to be used wherever a WithPath is expected without converting into ItemData.
impl<'db> WithPath for SubItemData<'db> {
    fn name(&self) -> &str {
        self.name.as_str()
    }
    fn full_path(&self) -> String {
        self.full_path.clone()
    }
    fn parent_full_path(&self) -> Option<String> {
        self.parent_full_path.clone()
    }
}
