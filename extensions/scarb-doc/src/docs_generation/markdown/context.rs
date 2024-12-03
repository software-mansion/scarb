use crate::docs_generation::markdown::traits::WithPath;
use crate::docs_generation::markdown::SUMMARY_FILENAME;
use crate::types::Crate;
use cairo_lang_defs::ids::{ImplItemId, LookupItemId, TraitItemId};
use cairo_lang_doc::documentable_item::DocumentableItemId;
use cairo_lang_doc::parser::CommentLinkToken;
use std::collections::HashMap;

pub type IncludedItems<'a> = HashMap<DocumentableItemId, &'a dyn WithPath>;

pub struct MarkdownGenerationContext<'a> {
    included_items: IncludedItems<'a>,
}

impl<'a> MarkdownGenerationContext<'a> {
    pub fn from_crate(crate_: &'a Crate) -> Self {
        let included_items = crate_.root_module.get_all_item_ids();
        Self {
            included_items: included_items
                .into_iter()
                .map(|(id, item)| {
                    let item: &dyn WithPath = item;
                    (id, item)
                })
                .collect(),
        }
    }

    pub fn resolve_markdown_file_path_from_link(&self, link: &CommentLinkToken) -> String {
        match link.resolved_item {
            Some(resolved_item_id) => match self.included_items.get(&resolved_item_id) {
                Some(resolved_item) => match resolved_item_id {
                    DocumentableItemId::Member(_)
                    | DocumentableItemId::Variant(_)
                    | DocumentableItemId::LookupItem(LookupItemId::TraitItem(TraitItemId::Type(
                        _,
                    )))
                    | DocumentableItemId::LookupItem(LookupItemId::TraitItem(
                        TraitItemId::Function(_),
                    ))
                    | DocumentableItemId::LookupItem(LookupItemId::TraitItem(
                        TraitItemId::Constant(_),
                    ))
                    | DocumentableItemId::LookupItem(LookupItemId::ImplItem(ImplItemId::Type(_)))
                    | DocumentableItemId::LookupItem(LookupItemId::ImplItem(
                        ImplItemId::Function(_),
                    ))
                    | DocumentableItemId::LookupItem(LookupItemId::ImplItem(
                        ImplItemId::Constant(_),
                    )) => {
                        match resolved_item.parent_full_path() {
                            Some(parent_path) => {
                                format!(
                                    "{}#{}",
                                    path_to_file_link(&parent_path),
                                    resolved_item.name().to_lowercase()
                                )
                            }
                            // Only root_module / crate doesn't have the parent.
                            _ => SUMMARY_FILENAME.to_string(),
                        }
                    }
                    _ => path_to_file_link(&resolved_item.full_path()),
                },
                None => link.path.clone().unwrap_or(link.label.clone()),
            },
            None => link.path.clone().unwrap_or(link.label.clone()),
        }
    }
}

fn path_to_file_link(path: &str) -> String {
    format!("./{}.md", path.replace("::", "-"))
}
