use crate::db::ScarbDocDatabase;
use cairo_lang_defs::ids::{
    ImplItemId, LookupItemId, ModuleItemId, TopLevelLanguageElementId, TraitItemId,
};
use cairo_lang_doc::documentable_item::DocumentableItemId;

fn get_documentable_full_path(
    db: &ScarbDocDatabase,
    item_id: DocumentableItemId,
) -> Option<String> {
    match item_id {
        DocumentableItemId::LookupItem(item_id) => match item_id {
            LookupItemId::ModuleItem(item_id) => match item_id {
                ModuleItemId::Struct(item_id) => Some(item_id.full_path(db).replace("::", "-")),
                ModuleItemId::Enum(item_id) => Some(item_id.full_path(db).replace("::", "-")),
                ModuleItemId::Constant(item_id) => Some(item_id.full_path(db).replace("::", "-")),
                ModuleItemId::FreeFunction(item_id) => {
                    Some(item_id.full_path(db).replace("::", "-"))
                }
                ModuleItemId::TypeAlias(item_id) => Some(item_id.full_path(db).replace("::", "-")),
                ModuleItemId::ImplAlias(item_id) => Some(item_id.full_path(db).replace("::", "-")),
                ModuleItemId::Trait(item_id) => Some(item_id.full_path(db).replace("::", "-")),
                ModuleItemId::Impl(item_id) => Some(item_id.full_path(db).replace("::", "-")),
                ModuleItemId::ExternType(item_id) => Some(item_id.full_path(db).replace("::", "-")),
                ModuleItemId::ExternFunction(item_id) => {
                    Some(item_id.full_path(db).replace("::", "-"))
                }
                ModuleItemId::Submodule(item_id) => Some(item_id.full_path(db).replace("::", "-")),
                ModuleItemId::Use(item_id) => Some(item_id.full_path(db).replace("::", "-")),
            },
            LookupItemId::TraitItem(item_id) => match item_id {
                TraitItemId::Function(item_id) => Some(item_id.full_path(db).replace("::", "-")),
                TraitItemId::Constant(item_id) => Some(item_id.full_path(db).replace("::", "-")),
                TraitItemId::Type(item_id) => Some(item_id.full_path(db).replace("::", "-")),
                TraitItemId::Impl(item_id) => Some(item_id.full_path(db).replace("::", "-")),
            },
            LookupItemId::ImplItem(item_id) => match item_id {
                ImplItemId::Function(item_id) => Some(item_id.full_path(db).replace("::", "-")),
                ImplItemId::Constant(item_id) => Some(item_id.full_path(db).replace("::", "-")),
                ImplItemId::Type(item_id) => Some(item_id.full_path(db).replace("::", "-")),
                ImplItemId::Impl(item_id) => Some(item_id.full_path(db).replace("::", "-")),
            },
        },
        DocumentableItemId::Member(item_id) => Some(item_id.full_path(db).replace("::", "-")),
        DocumentableItemId::Variant(item_id) => Some(item_id.full_path(db).replace("::", "-")),
        DocumentableItemId::Crate(_) => None,
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DocLocationLink {
    pub start: usize,
    pub end: usize,
    pub full_path: Option<String>,
}

impl DocLocationLink {
    pub fn new(
        start: usize,
        end: usize,
        item_id: DocumentableItemId,
        db: &ScarbDocDatabase,
    ) -> Self {
        Self {
            start,
            end,
            full_path: get_documentable_full_path(db, item_id),
        }
    }
}
