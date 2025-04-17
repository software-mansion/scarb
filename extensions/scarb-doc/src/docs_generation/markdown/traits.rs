use super::context::MarkdownGenerationContext;
use crate::docs_generation::markdown::{
    SHORT_DOCUMENTATION_AVOID_PREFIXES, SHORT_DOCUMENTATION_LEN,
};
use crate::docs_generation::{DocItem, PrimitiveDocItem, TopLevelDocItem};
use crate::location_links::DocLocationLink;
use crate::types::{
    Constant, Enum, ExternFunction, ExternType, FreeFunction, Impl, ImplAlias, ItemData, Module,
    Struct, Trait, TypeAlias,
};
use anyhow::Result;
use cairo_lang_doc::parser::DocumentationCommentToken;
use itertools::Itertools;
use std::collections::HashMap;
use std::fmt::Write;

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

pub trait MarkdownDocItem: DocItem {
    fn generate_markdown(
        &self,
        context: &MarkdownGenerationContext,
        header_level: usize,
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
                        let file_path = context.resolve_markdown_file_path_from_link(link);
                        format!("[{}]({})", link.label.clone(), file_path).replace("\n", " ")
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
                        let file_path = context.resolve_markdown_file_path_from_link(link);
                        format!("[{}]({})", link.label.clone(), file_path)
                    }
                })
                .join("")
        })
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
    ) -> Result<String> {
        generate_markdown_from_item_data(self, context, header_level)
    }
}

impl MarkdownDocItem for Enum {
    fn generate_markdown(
        &self,
        context: &MarkdownGenerationContext,
        header_level: usize,
    ) -> Result<String> {
        let mut markdown = generate_markdown_from_item_data(self, context, header_level)?;

        markdown += &generate_markdown_for_subitems(&self.variants, context, header_level)?;

        Ok(markdown)
    }
}

impl MarkdownDocItem for Impl {
    fn generate_markdown(
        &self,
        context: &MarkdownGenerationContext,
        header_level: usize,
    ) -> Result<String> {
        let mut markdown = generate_markdown_from_item_data(self, context, header_level)?;

        markdown += &generate_markdown_for_subitems(&self.impl_constants, context, header_level)?;
        markdown += &generate_markdown_for_subitems(&self.impl_functions, context, header_level)?;
        markdown += &generate_markdown_for_subitems(&self.impl_types, context, header_level)?;

        Ok(markdown)
    }
}

impl MarkdownDocItem for Module {
    fn generate_markdown(
        &self,
        context: &MarkdownGenerationContext,
        header_level: usize,
    ) -> Result<String> {
        let mut markdown = generate_markdown_from_item_data(self, context, header_level)?;

        markdown += &generate_markdown_table_summary_for_top_level_subitems(
            &self.submodules.iter().collect_vec(),
            context,
            &self.markdown_formatted_path(),
        )?;
        markdown += &generate_markdown_table_summary_for_top_level_subitems(
            &self.constants.iter().collect_vec(),
            context,
            &self.markdown_formatted_path(),
        )?;
        markdown += &generate_markdown_table_summary_for_top_level_subitems(
            &self.free_functions.iter().collect_vec(),
            context,
            &self.markdown_formatted_path(),
        )?;
        markdown += &generate_markdown_table_summary_for_top_level_subitems(
            &self.structs.iter().collect_vec(),
            context,
            &self.markdown_formatted_path(),
        )?;
        markdown += &generate_markdown_table_summary_for_top_level_subitems(
            &self.enums.iter().collect_vec(),
            context,
            &self.markdown_formatted_path(),
        )?;
        markdown += &generate_markdown_table_summary_for_top_level_subitems(
            &self.type_aliases.iter().collect_vec(),
            context,
            &self.markdown_formatted_path(),
        )?;
        markdown += &generate_markdown_table_summary_for_top_level_subitems(
            &self.impl_aliases.iter().collect_vec(),
            context,
            &self.markdown_formatted_path(),
        )?;
        markdown += &generate_markdown_table_summary_for_top_level_subitems(
            &self.traits.iter().collect_vec(),
            context,
            &self.markdown_formatted_path(),
        )?;
        markdown += &generate_markdown_table_summary_for_top_level_subitems(
            &self.impls.iter().collect_vec(),
            context,
            &self.markdown_formatted_path(),
        )?;
        markdown += &generate_markdown_table_summary_for_top_level_subitems(
            &self.extern_types.iter().collect_vec(),
            context,
            &self.markdown_formatted_path(),
        )?;
        markdown += &generate_markdown_table_summary_for_top_level_subitems(
            &self.extern_functions.iter().collect_vec(),
            context,
            &self.markdown_formatted_path(),
        )?;

        Ok(markdown)
    }
}

impl MarkdownDocItem for Struct {
    fn generate_markdown(
        &self,
        context: &MarkdownGenerationContext,
        header_level: usize,
    ) -> Result<String> {
        let mut markdown = generate_markdown_from_item_data(self, context, header_level)?;

        markdown += &generate_markdown_for_subitems(&self.members, context, header_level)?;

        Ok(markdown)
    }
}

impl MarkdownDocItem for Trait {
    fn generate_markdown(
        &self,
        context: &MarkdownGenerationContext,
        header_level: usize,
    ) -> Result<String> {
        let mut markdown = generate_markdown_from_item_data(self, context, header_level)?;

        markdown += &generate_markdown_for_subitems(&self.trait_constants, context, header_level)?;
        markdown += &generate_markdown_for_subitems(&self.trait_functions, context, header_level)?;
        markdown += &generate_markdown_for_subitems(&self.trait_types, context, header_level)?;

        Ok(markdown)
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
    markdown_formatted_path: &String,
) -> Result<String> {
    let mut markdown = String::new();

    if !subitems.is_empty() {
        let linked = format!(
            "[{}](./{}-{})",
            T::HEADER,
            markdown_formatted_path,
            T::ITEMS_SUMMARY_FILENAME
        );

        writeln!(&mut markdown, "\n{}\n ---\n| | |\n|:---|:---|", linked,)?;

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

fn generate_markdown_for_subitems<T: MarkdownDocItem + PrimitiveDocItem>(
    subitems: &[T],
    context: &MarkdownGenerationContext,
    header_level: usize,
) -> Result<String> {
    let mut markdown = String::new();

    if !subitems.is_empty() {
        let header = str::repeat("#", header_level + 1);

        writeln!(&mut markdown, "{header} {}\n", T::HEADER)?;
        for item in subitems {
            writeln!(
                &mut markdown,
                "{}",
                item.generate_markdown(context, header_level + 2)?
            )?;
        }
    }

    Ok(markdown)
}

fn generate_markdown_from_item_data(
    doc_item: &impl MarkdownDocItem,
    context: &MarkdownGenerationContext,
    header_level: usize,
) -> Result<String> {
    let mut markdown = String::new();

    let header = str::repeat("#", header_level);

    writeln!(&mut markdown, "{header} {}\n", doc_item.name())?;

    if let Some(doc) = doc_item.get_documentation(context) {
        writeln!(&mut markdown, "{doc}\n")?;
    }

    writeln!(
        &mut markdown,
        "Fully qualified path: {}\n",
        get_linked_path(doc_item.full_path())
    )?;

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
