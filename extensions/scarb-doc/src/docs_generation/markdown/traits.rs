use super::context::MarkdownGenerationContext;
use crate::docs_generation::markdown::{
    BASE_MODULE_CHAPTER_PREFIX, GROUP_CHAPTER_PREFIX, SHORT_DOCUMENTATION_AVOID_PREFIXES,
    SHORT_DOCUMENTATION_LEN,
};
use crate::docs_generation::{DocItem, PrimitiveDocItem, SubPathDocItem, TopLevelDocItem};
use crate::location_links::DocLocationLink;
use crate::types::groups::Group;
use crate::types::module_type::{Module, ModulePubUses};
use crate::types::other_types::{
    Constant, Enum, ExternFunction, ExternType, FreeFunction, Impl, ImplAlias, ImplConstant,
    ImplFunction, ImplType, ItemData, Member, Struct, Trait, TraitConstant, TraitFunction,
    TraitType, TypeAlias, Variant,
};
use anyhow::Result;
use cairo_lang_doc::parser::{CommentLinkToken, DocumentationCommentToken};
use itertools::Itertools;
use std::collections::HashMap;
use std::fmt::Write;
use std::option::Option;

const RE_EXPORTS_CHAPTER: &str = "\n\n---\n \n# Re-exports: \n";
const GROUPS_CHAPTER: &str = "\n\n---\n \n# Groups: \n";

pub trait TopLevelMarkdownDocItem: MarkdownDocItem + TopLevelDocItem {
    const ITEMS_SUMMARY_FILENAME: &'static str;

    fn filename(&self) -> String {
        format!("{}.md", self.markdown_formatted_path())
    }

    fn md_ref(&self, relative_path: Option<String>) -> String {
        match relative_path {
            Some(path) => format!("[{}](./{})", path, self.filename()),
            None => format!("[{}](./{})", self.name(), self.filename()),
        }
    }

    fn generate_markdown_nested_list_item(
        &self,
        relative_path: Option<String>,
        nesting_level: usize,
    ) -> String {
        format!(
            "{}- {}",
            "  ".repeat(nesting_level),
            self.md_ref(relative_path)
        )
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

macro_rules! impl_markdown_doc_item {
    ($ty:ty) => {
        impl MarkdownDocItem for $ty {
            fn generate_markdown(
                &self,
                context: &MarkdownGenerationContext,
                header_level: usize,
                item_suffix: Option<usize>,
            ) -> Result<String> {
                generate_markdown_from_item_data(self, context, header_level, item_suffix)
            }

            fn get_full_path(&self, item_suffix: Option<usize>) -> String {
                get_full_subitem_path(self, item_suffix)
            }
        }
    };
}

impl_markdown_doc_item!(Member);
impl_markdown_doc_item!(ImplFunction);
impl_markdown_doc_item!(ImplType);
impl_markdown_doc_item!(TraitFunction);
impl_markdown_doc_item!(Variant);
impl_markdown_doc_item!(ImplConstant);
impl_markdown_doc_item!(TraitConstant);
impl_markdown_doc_item!(TraitType);

pub trait MarkdownDocItem: DocItem {
    fn generate_markdown(
        &self,
        context: &MarkdownGenerationContext,
        header_level: usize,
        item_suffix: Option<usize>,
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
                            return short_doc_buff.trim().to_string();
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
                    return short_doc_buff.trim().to_string();
                } else {
                    short_doc_buff.push_str(&text_formatted);
                }
            }
        }
        short_doc_buff.trim().to_string()
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

    fn get_full_path(&self, _item_suffix: Option<usize>) -> String {
        get_linked_path(self.full_path())
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
    ) -> Result<String> {
        generate_markdown_from_item_data(self, context, header_level, None)
    }
}

impl MarkdownDocItem for Enum {
    fn generate_markdown(
        &self,
        context: &MarkdownGenerationContext,
        header_level: usize,
        _item_suffix: Option<usize>,
    ) -> Result<String> {
        let mut markdown = generate_markdown_from_item_data(self, context, header_level, None)?;
        let mut suffix_calculator = ItemSuffixCalculator::new(self.name());
        markdown += &generate_markdown_for_subitems(
            &self.variants,
            context,
            header_level,
            &mut suffix_calculator,
        )?;

        Ok(markdown)
    }
}

impl MarkdownDocItem for Impl {
    fn generate_markdown(
        &self,
        context: &MarkdownGenerationContext,
        header_level: usize,
        _item_suffix: Option<usize>,
    ) -> Result<String> {
        let mut markdown = generate_markdown_from_item_data(self, context, header_level, None)?;
        let mut suffix_calculator = ItemSuffixCalculator::new(self.name());

        markdown += &generate_markdown_for_subitems(
            &self.impl_constants,
            context,
            header_level,
            &mut suffix_calculator,
        )?;

        markdown += &generate_markdown_for_subitems(
            &self.impl_functions,
            context,
            header_level,
            &mut suffix_calculator,
        )?;

        markdown += &generate_markdown_for_subitems(
            &self.impl_types,
            context,
            header_level,
            &mut suffix_calculator,
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

    if !buff.is_empty() {
        return format!("{RE_EXPORTS_CHAPTER}{buff}");
    }
    buff
}

impl MarkdownDocItem for Module {
    fn generate_markdown(
        &self,
        context: &MarkdownGenerationContext,
        header_level: usize,
        _item_suffix: Option<usize>,
    ) -> Result<String> {
        let mut markdown = generate_markdown_from_item_data(self, context, header_level, None)?;

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

impl MarkdownDocItem for Struct {
    fn generate_markdown(
        &self,
        context: &MarkdownGenerationContext,
        header_level: usize,
        _item_suffix: Option<usize>,
    ) -> Result<String> {
        let mut markdown = generate_markdown_from_item_data(self, context, header_level, None)?;

        let mut suffix_calculator = ItemSuffixCalculator::new(self.name());
        markdown += &generate_markdown_for_subitems(
            &self.members,
            context,
            header_level,
            &mut suffix_calculator,
        )?;

        Ok(markdown)
    }
}

impl MarkdownDocItem for Trait {
    fn generate_markdown(
        &self,
        context: &MarkdownGenerationContext,
        header_level: usize,
        _item_suffix: Option<usize>,
    ) -> Result<String> {
        let mut markdown = generate_markdown_from_item_data(self, context, header_level, None)?;
        let mut suffix_calculator = ItemSuffixCalculator::new(self.name());

        markdown += &generate_markdown_for_subitems(
            &self.trait_constants,
            context,
            header_level,
            &mut suffix_calculator,
        )?;
        markdown += &generate_markdown_for_subitems(
            &self.trait_functions,
            context,
            header_level,
            &mut suffix_calculator,
        )?;
        markdown += &generate_markdown_for_subitems(
            &self.trait_types,
            context,
            header_level,
            &mut suffix_calculator,
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
            "[{}](./{}-{})",
            T::HEADER,
            module_name,
            T::ITEMS_SUMMARY_FILENAME
        );

        writeln!(
            &mut markdown,
            "\n{} {}\n\n| | |\n|:---|:---|",
            prefix, linked,
        )?;

        let items_with_relative_path = mark_duplicated_item_with_relative_path(subitems);
        for (item, relative_path) in items_with_relative_path {
            let item_doc = item.get_short_documentation(context);
            writeln!(
                &mut markdown,
                "| {} | {}[...](./{}) |",
                item.md_ref(relative_path),
                item_doc,
                item.filename(),
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
            markdown += &format!("\n## [{}]({})\n", group.name, group.filename(),);

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
                "| {} | {}[...](./{}) |",
                item.md_ref(relative_path),
                item_doc,
                item.filename(),
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
                item.generate_markdown(context, header_level + 2, postfix)?
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
) -> Result<String> {
    let mut markdown = String::new();

    let header = str::repeat("#", header_level);

    writeln!(&mut markdown, "{header} {}\n", doc_item.name())?;

    if let Some(doc) = doc_item.get_documentation(context) {
        writeln!(&mut markdown, "{doc}\n")?;
    }

    let full_path = doc_item.get_full_path(item_suffix);
    writeln!(&mut markdown, "Fully qualified path: {full_path}\n",)?;

    if let Some(group_name) = doc_item.group_name() {
        let group_path = format!("[{}](./{}.md)", group_name, group_name.replace(" ", "_"),);
        writeln!(&mut markdown, "Part of the group: {group_path}\n",)?;
    }

    if let Some(sig) = &doc_item.signature() {
        if !sig.is_empty() {
            writeln!(
                &mut markdown,
                "<pre><code class=\"language-cairo\">{}</code></pre>\n",
                format_signature(sig, doc_item.doc_location_links())
            )?;
        }
    }
    Ok(markdown)
}

fn get_linked_path(full_path: &str) -> String {
    let path_items = full_path.split("::").collect::<Vec<_>>();
    let mut result: Vec<String> = Vec::new();
    let mut current_path = String::new();
    for element in path_items {
        if !current_path.is_empty() {
            current_path.push('-');
        }
        current_path.push_str(element);
        let formatted = format!("[{}](./{}.md)", element, current_path);
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
) -> String {
    if let Some((parent_path, item_path)) = item.full_path().rsplit_once("::") {
        let last_path = format!(
            "{}.md#{}{}",
            parent_path.replace("::", "-"),
            item_path.to_lowercase(),
            if let Some(item_suffix) = item_suffix {
                format!("-{}", item_suffix)
            } else {
                "".to_string()
            }
        );
        format!(
            "{}::[{}](./{})",
            get_linked_path(parent_path),
            &item_path,
            last_path
        )
    } else {
        get_linked_path(item.full_path())
    }
}

fn format_signature(input: &str, links: &[DocLocationLink]) -> String {
    let mut escaped = String::with_capacity(input.len());
    let mut index_pointer = 0;

    let sorted_links = links.iter().sorted_by_key(|k| k.start).collect_vec();
    let mut chars_iter = input.chars().enumerate();
    let mut skip_chars = 0;

    while index_pointer < input.len() {
        if let Some((i, ch)) = chars_iter.nth(skip_chars) {
            skip_chars = 0;

            if let Some(link) = sorted_links
                .iter()
                .find(|&link| i >= link.start && i < link.end)
            {
                let slice = escape_html(&input[link.start..link.end]);
                escaped.push_str(&format!(
                    "<a href=\"{}.html\">{}</a>",
                    link.full_path, slice
                ));
                index_pointer = link.end;
                skip_chars = link.end - link.start - 1;
                continue;
            } else {
                escaped.push_str(&escape_html_char(ch));
                index_pointer += ch.len_utf8();
            }
        } else {
            break;
        }
    }
    escaped
}

fn escape_html(input: &str) -> String {
    input.chars().map(escape_html_char).collect::<String>()
}

fn escape_html_char(ch: char) -> String {
    match ch {
        '<' => "&lt;".to_string(),
        '>' => "&gt;".to_string(),
        '"' => "&quot;".to_string(),
        '&' => "&amp;".to_string(),
        '\'' => "&apos;".to_string(),
        _ => ch.to_string(),
    }
}

pub trait WithPath {
    fn name(&self) -> &str;
    fn full_path(&self) -> String;
    fn parent_full_path(&self) -> Option<String>;
}

pub trait WithItemData {
    fn item_data(&self) -> &ItemData;
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

impl WithItemData for ItemData {
    fn item_data(&self) -> &ItemData {
        self
    }
}
