use crate::attributes::find_groups_from_attributes;
use crate::db::ScarbDocDatabase;
use crate::location_links::DocLocationLink;
use crate::types::other_types::doc_full_path;
use cairo_lang_defs::ids::{ModuleId, TopLevelLanguageElementId};
use cairo_lang_doc::db::DocGroup;
use cairo_lang_doc::documentable_item::DocumentableItemId;
use cairo_lang_doc::parser::DocumentationCommentToken;
use cairo_lang_filesystem::ids::CrateId;
use serde::Serialize;
use serde::Serializer;
use std::fmt::Debug;

#[derive(Debug, Serialize, Clone)]
pub struct ItemData<'db> {
    #[serde(skip_serializing)]
    pub id: DocumentableItemId<'db>,
    #[serde(skip_serializing)]
    pub parent_full_path: Option<String>,
    pub name: String,
    #[serde(serialize_with = "documentation_serializer")]
    pub doc: Option<Vec<DocumentationCommentToken<'db>>>,
    pub signature: Option<String>,
    pub full_path: String,
    #[serde(skip_serializing)]
    pub doc_location_links: Vec<DocLocationLink>,
    pub group: Option<String>,
}

impl<'db> ItemData<'db> {
    pub fn new(
        db: &'db ScarbDocDatabase,
        id: impl TopLevelLanguageElementId<'db>,
        documentable_item_id: DocumentableItemId<'db>,
        parent_full_path: String,
    ) -> Self {
        let (signature, doc_location_links) =
            db.get_item_signature_with_links(documentable_item_id);
        let doc_location_links = doc_location_links
            .iter()
            .map(|link| DocLocationLink::new(link.start, link.end, link.item_id, db))
            .collect::<Vec<_>>();
        let group = find_groups_from_attributes(db, &id);
        Self {
            id: documentable_item_id,
            name: id.name(db).to_string(db),
            doc: db.get_item_documentation_as_tokens(documentable_item_id),
            signature,
            full_path: format!("{}::{}", parent_full_path, id.name(db).long(db)),
            parent_full_path: Some(parent_full_path),
            doc_location_links,
            group,
        }
    }

    pub fn new_without_signature(
        db: &'db ScarbDocDatabase,
        id: impl TopLevelLanguageElementId<'db>,
        documentable_item_id: DocumentableItemId<'db>,
    ) -> Self {
        Self {
            id: documentable_item_id,
            name: id.name(db).to_string(db),
            doc: db.get_item_documentation_as_tokens(documentable_item_id),
            signature: None,
            full_path: format!(
                "{}::{}",
                doc_full_path(&id.parent_module(db), db),
                id.name(db).long(db)
            ),
            parent_full_path: Some(doc_full_path(&id.parent_module(db), db)),
            doc_location_links: vec![],
            group: find_groups_from_attributes(db, &id),
        }
    }

    pub fn new_crate(db: &'db ScarbDocDatabase, id: CrateId<'db>) -> Self {
        let documentable_id = DocumentableItemId::Crate(id);
        Self {
            id: documentable_id,
            name: id.long(db).name().to_string(db),
            doc: db.get_item_documentation_as_tokens(documentable_id),
            signature: None,
            full_path: ModuleId::CrateRoot(id).full_path(db),
            parent_full_path: None,
            doc_location_links: vec![],
            group: None,
        }
    }
}

/// Helper struct for custom serialization of [`ItemData`] for [`crate::types::other_types::Variant`] and [`crate::types::other_types::Member`].
#[derive(Debug, Serialize, Clone)]
pub struct SubItemData<'db> {
    #[serde(skip_serializing)]
    pub id: DocumentableItemId<'db>,
    #[serde(skip_serializing)]
    pub parent_full_path: Option<String>,
    pub name: String,
    #[serde(serialize_with = "documentation_serializer")]
    pub doc: Option<Vec<DocumentationCommentToken<'db>>>,
    pub signature: Option<String>,
    pub full_path: String,
    #[serde(skip_serializing)]
    pub doc_location_links: Vec<DocLocationLink>,
    #[serde(skip_serializing)]
    pub group: Option<String>,
}

impl<'db> From<SubItemData<'db>> for ItemData<'db> {
    fn from(val: SubItemData<'db>) -> Self {
        ItemData {
            id: val.id,
            parent_full_path: val.parent_full_path,
            name: val.name,
            doc: val.doc,
            signature: val.signature,
            full_path: val.full_path,
            doc_location_links: val.doc_location_links,
            group: val.group,
        }
    }
}

impl<'db> From<ItemData<'db>> for SubItemData<'db> {
    fn from(val: ItemData<'db>) -> Self {
        SubItemData {
            id: val.id,
            parent_full_path: val.parent_full_path,
            name: val.name,
            doc: val.doc,
            signature: val.signature,
            full_path: val.full_path,
            doc_location_links: val.doc_location_links,
            group: val.group,
        }
    }
}

fn documentation_serializer<S>(
    docs: &Option<Vec<DocumentationCommentToken>>,
    serializer: S,
) -> anyhow::Result<S::Ok, S::Error>
where
    S: Serializer,
{
    match docs {
        Some(doc_vec) => {
            let combined = doc_vec
                .iter()
                .map(|dct| dct.to_string())
                .collect::<Vec<String>>();
            serializer.serialize_str(&combined.join(""))
        }
        None => serializer.serialize_none(),
    }
}
