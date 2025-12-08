use crate::db::ScarbDocDatabase;
use crate::docs_generation::markdown::context::IncludedItems;
use crate::docs_generation::markdown::traits::WithPath;
use crate::types::item_data::{ItemData, SubItemData};
use crate::types::module_type::is_doc_hidden_attr;
use crate::types::other_types::doc_full_path;
use cairo_lang_defs::ids::{
    LanguageElementId, LookupItemId, MemberId, ModuleItemId, NamedLanguageElementId, StructId,
};
use cairo_lang_diagnostics::Maybe;
use cairo_lang_doc::documentable_item::DocumentableItemId;
use cairo_lang_semantic::items::structure::StructSemantic;
use cairo_lang_semantic::items::visibility::Visibility;
use cairo_lang_syntax::node::ast;
use serde::Serialize;

#[derive(Serialize, Clone)]
pub struct Struct<'db> {
    #[serde(skip)]
    pub id: StructId<'db>,
    #[serde(skip)]
    pub node: ast::ItemStructPtr<'db>,
    pub members: Vec<Member<'db>>,
    pub item_data: ItemData<'db>,
}

impl<'db> Struct<'db> {
    pub fn new(
        db: &'db ScarbDocDatabase,
        id: StructId<'db>,
        include_private_items: bool,
    ) -> Maybe<Self> {
        let members = db.struct_members(id)?;

        let item_data = ItemData::new(
            db,
            id,
            LookupItemId::ModuleItem(ModuleItemId::Struct(id)).into(),
            doc_full_path(&id.parent_module(db), db),
        );
        let members = members
            .iter()
            .filter_map(|(_, semantic_member)| {
                let visible = matches!(semantic_member.visibility, Visibility::Public);
                let syntax_node = &semantic_member.id.stable_location(db).syntax_node(db);
                if (include_private_items || visible) && !is_doc_hidden_attr(db, syntax_node) {
                    Some(Ok(Member::new(db, semantic_member.id)))
                } else {
                    None
                }
            })
            .collect::<Maybe<Vec<_>>>()?;

        let node = id.stable_ptr(db);
        Ok(Self {
            id,
            node,
            members,
            item_data,
        })
    }

    pub fn get_all_item_ids<'a>(&'a self) -> IncludedItems<'a, 'db> {
        self.members
            .iter()
            .map(|item| (item.item_data.id, &item.item_data as &dyn WithPath))
            .collect()
    }
}

#[derive(Serialize, Clone)]
pub struct Member<'db> {
    #[serde(skip)]
    pub id: MemberId<'db>,
    #[serde(skip)]
    pub node: ast::MemberPtr<'db>,
    pub item_data: SubItemData<'db>,
}

impl<'db> Member<'db> {
    pub fn new(db: &'db ScarbDocDatabase, id: MemberId<'db>) -> Self {
        let node = id.stable_ptr(db);

        let parent_path = format!(
            "{}::{}",
            doc_full_path(&id.parent_module(db), db),
            id.struct_id(db).name(db).to_string(db)
        );
        Self {
            id,
            node,
            item_data: ItemData::new(db, id, DocumentableItemId::Member(id), parent_path).into(),
        }
    }
}
