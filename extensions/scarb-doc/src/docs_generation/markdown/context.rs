use crate::doc_test::runner::ExecutionResults;
use crate::docs_generation::common::{OutputFilesExtension, SummaryIndexMap};
use crate::docs_generation::markdown::SUMMARY_FILENAME;
use crate::docs_generation::markdown::traits::WithItemDataCommon;
use crate::linking::RemoteDocLinkingData;
use crate::location_links::DocLocationLink;
use crate::types::crate_type::Crate;
use cairo_lang_defs::ids::{ImplItemId, LookupItemId, TraitItemId};
use cairo_lang_doc::documentable_item::DocumentableItemId;
use itertools::Itertools;
use std::collections::HashMap;

pub type IncludedItems<'a, 'db> = HashMap<DocumentableItemId<'db>, &'a dyn WithItemDataCommon>;

pub struct MarkdownGenerationContext<'a, 'db> {
    included_items: IncludedItems<'a, 'db>,
    formatting: Box<dyn Formatting>,
    pub(crate) files_extension: &'static str,
    /// Data necessary for linking to the remote repository.
    pub remote_linking_data: RemoteDocLinkingData,
    execution_results: Option<ExecutionResults>,
}

pub trait Formatting {
    fn header(&self, header_level: usize, name: &str, full_path: &str) -> String;
    fn signature(
        &self,
        signature: &str,
        location_links: &[DocLocationLink],
        summary_index_map: &SummaryIndexMap,
    ) -> String;
    fn fully_qualified_path(&self, full_path: String) -> Option<String>;
    fn group(&self, group_name: &str) -> String;

    fn header_primitive(&self, header_level: usize, name: &str, full_path: &str) -> String;
}

pub struct MdxFormatting;
pub struct MarkdownFormatting;

impl Formatting for MdxFormatting {
    fn header(&self, _header_level: usize, _name: &str, full_path: &str) -> String {
        format!("---\n title: \"{}\"\n---", full_path,)
    }

    fn signature(
        &self,
        signature: &str,
        _location_links: &[DocLocationLink],
        _summary_index_map: &SummaryIndexMap,
    ) -> String {
        format!("## Signature\n\n```rust\n{signature}\n```\n")
    }

    fn fully_qualified_path(&self, _full_path: String) -> Option<String> {
        None
    }

    fn group(&self, group_name: &str) -> String {
        format!("## Group\n{group_name}\n\n")
    }

    fn header_primitive(&self, _header_level: usize, name: &str, _full_path: &str) -> String {
        format!("## {name}")
    }
}

impl Formatting for MarkdownFormatting {
    fn header(&self, header_level: usize, name: &str, _full_path: &str) -> String {
        let header = str::repeat("#", header_level);
        format!("{header} {}", name)
    }
    fn signature(
        &self,
        signature: &str,
        location_links: &[DocLocationLink],
        summary_index_map: &SummaryIndexMap,
    ) -> String {
        format!(
            "<pre><code class=\"language-cairo\">{}</code></pre>\n",
            format_signature(
                signature,
                location_links,
                summary_index_map,
                OutputFilesExtension::Md.get_string()
            )
        )
    }

    fn fully_qualified_path(&self, full_path: String) -> Option<String> {
        Some(format!("Fully qualified path: {full_path}",))
    }

    fn group(&self, group_name: &str) -> String {
        let group_path = format!(
            "[{}](./{}{})",
            group_name,
            &group_name.replace(" ", "_"),
            OutputFilesExtension::Md.get_string()
        );
        format!("Part of the group: {group_path}\n")
    }

    fn header_primitive(&self, header_level: usize, name: &str, full_path: &str) -> String {
        self.header(header_level, name, full_path)
    }
}

impl<'a, 'db> MarkdownGenerationContext<'a, 'db> {
    pub fn from_crate(
        crate_: &'a Crate<'db>,
        format: OutputFilesExtension,
        remote_linking_data: RemoteDocLinkingData,
        execution_results: Option<ExecutionResults>,
    ) -> Self
    where
        'a: 'db,
    {
        let formatting: Box<dyn Formatting> = match format {
            OutputFilesExtension::Mdx => Box::new(MdxFormatting),
            OutputFilesExtension::Md => Box::new(MarkdownFormatting),
            _ => panic!("Json should not be used for markdown generation."),
        };

        let included_items = crate_.root_module.get_all_item_ids();
        Self {
            included_items,
            formatting,
            files_extension: format.get_string(),
            remote_linking_data,
            execution_results,
        }
    }

    pub fn resolve_markdown_file_path_from_item(
        &self,
        resolved_item_id: DocumentableItemId<'db>,
    ) -> Option<String> {
        let resolved_item = self.included_items.get(&resolved_item_id)?;
        match resolved_item_id {
            DocumentableItemId::Member(_)
            | DocumentableItemId::Variant(_)
            | DocumentableItemId::LookupItem(LookupItemId::TraitItem(TraitItemId::Type(_)))
            | DocumentableItemId::LookupItem(LookupItemId::TraitItem(TraitItemId::Function(_)))
            | DocumentableItemId::LookupItem(LookupItemId::TraitItem(TraitItemId::Constant(_)))
            | DocumentableItemId::LookupItem(LookupItemId::ImplItem(ImplItemId::Type(_)))
            | DocumentableItemId::LookupItem(LookupItemId::ImplItem(ImplItemId::Function(_)))
            | DocumentableItemId::LookupItem(LookupItemId::ImplItem(ImplItemId::Constant(_))) => {
                match resolved_item.parent_full_path() {
                    Some(parent_path) => Some(format!(
                        "{}#{}",
                        path_to_file_link(&parent_path, self.files_extension),
                        resolved_item.name().to_lowercase()
                    )),
                    // Only root_module / crate doesn't have the parent.
                    _ => Some(format!("{SUMMARY_FILENAME}{}", self.files_extension)),
                }
            }
            _ => Some(path_to_file_link(
                &resolved_item.full_path(),
                self.files_extension,
            )),
        }
    }

    pub fn get_header(&self, header_level: usize, name: &str, full_path: &str) -> String {
        self.formatting.header(header_level, name, full_path)
    }

    pub fn get_signature(
        &self,
        signature: &str,
        location_links: &[DocLocationLink],
        summary_index_map: &SummaryIndexMap,
    ) -> String {
        self.formatting
            .signature(signature, location_links, summary_index_map)
    }

    pub fn get_fully_qualified_path(&self, full_path: String) -> Option<String> {
        self.formatting.fully_qualified_path(full_path)
    }

    pub fn get_group(&self, group_name: &str) -> String {
        self.formatting.group(group_name)
    }

    pub fn get_header_primitive(&self, header_level: usize, name: &str, full_path: &str) -> String {
        self.formatting
            .header_primitive(header_level, name, full_path)
    }

    pub fn execution_results(&self) -> Option<&ExecutionResults> {
        self.execution_results.as_ref()
    }
}

pub fn path_to_file_link(path: &str, files_extension: &str) -> String {
    format!("./{}{files_extension}", path.replace("::", "-"))
}

fn format_signature(
    input: &str,
    links: &[DocLocationLink],
    index_map: &SummaryIndexMap,
    files_extension: &str,
) -> String {
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
                if index_map.contains_key(&format!("./{}{files_extension}", &link.full_path)) {
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
