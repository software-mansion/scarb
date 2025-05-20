use crate::db::ScarbDocDatabase;
use cairo_lang_defs::ids::{
    ImplItemId, LookupItemId, ModuleItemId, TopLevelLanguageElementId, TraitItemId,
};
use cairo_lang_doc::documentable_item::DocumentableItemId;

fn get_documentable_full_path(db: &ScarbDocDatabase, item_id: DocumentableItemId) -> String {
    match item_id {
        DocumentableItemId::LookupItem(item_id) => match item_id {
            LookupItemId::ModuleItem(item_id) => match item_id {
                ModuleItemId::Struct(item_id) => item_id.full_path(db),
                ModuleItemId::Enum(item_id) => item_id.full_path(db),
                ModuleItemId::Constant(item_id) => item_id.full_path(db),
                ModuleItemId::FreeFunction(item_id) => item_id.full_path(db),
                ModuleItemId::TypeAlias(item_id) => item_id.full_path(db),
                ModuleItemId::ImplAlias(item_id) => item_id.full_path(db),
                ModuleItemId::Trait(item_id) => item_id.full_path(db),
                ModuleItemId::Impl(item_id) => item_id.full_path(db),
                ModuleItemId::ExternType(item_id) => item_id.full_path(db),
                ModuleItemId::ExternFunction(item_id) => item_id.full_path(db),
                ModuleItemId::Submodule(item_id) => item_id.full_path(db),
                ModuleItemId::Use(item_id) => item_id.full_path(db),
                ModuleItemId::MacroDeclaration(item_id) => item_id.full_path(db),
            },
            LookupItemId::TraitItem(item_id) => match item_id {
                TraitItemId::Function(item_id) => item_id.full_path(db),
                TraitItemId::Constant(item_id) => item_id.full_path(db),
                TraitItemId::Type(item_id) => item_id.full_path(db),
                TraitItemId::Impl(item_id) => item_id.full_path(db),
            },
            LookupItemId::ImplItem(item_id) => match item_id {
                ImplItemId::Function(item_id) => item_id.full_path(db),
                ImplItemId::Constant(item_id) => item_id.full_path(db),
                ImplItemId::Type(item_id) => item_id.full_path(db),
                ImplItemId::Impl(item_id) => item_id.full_path(db),
            },
        },
        DocumentableItemId::Member(item_id) => item_id.full_path(db),
        DocumentableItemId::Variant(item_id) => item_id.full_path(db),
        DocumentableItemId::Crate(_) => "".to_string(),
    }
    .replace("::", "-")
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DocLocationLink {
    pub start: usize,
    pub end: usize,
    pub full_path: String,
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
