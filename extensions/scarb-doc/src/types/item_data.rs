use crate::attributes::find_groups_from_attributes;
use crate::db::ScarbDocDatabase;
use crate::doc_test::code_blocks::{CodeBlock, collect_code_blocks_from_tokens};
use crate::doc_link_resolver::resolve_linked_item;
use crate::location_links::DocLocationLink;
use crate::types::other_types::doc_full_path;
use cairo_lang_defs::db::DefsGroup;
use cairo_lang_defs::ids::{ModuleId, TopLevelLanguageElementId};
use cairo_lang_doc::db::DocGroup;
use cairo_lang_doc::documentable_item::DocumentableItemId;
use cairo_lang_doc::parser::DocumentationCommentToken;
use cairo_lang_filesystem::db::get_originating_location;
use cairo_lang_filesystem::ids::CrateId;
use serde::Serialize;
use serde::Serializer;
use std::collections::HashMap;
use std::fmt::Debug;
use std::ops::Range;

#[derive(Debug, Serialize, Clone)]
pub struct ItemData<'db> {
    #[serde(skip_serializing)]
    pub id: DocumentableItemId<'db>,
    #[serde(skip_serializing)]
    pub parent_full_path: Option<String>,
    pub name: String,
    #[serde(serialize_with = "documentation_serializer")]
    pub doc: Option<Vec<DocumentationCommentToken>>,
    pub signature: Option<String>,
    pub full_path: String,
    #[serde(skip_serializing)]
    pub code_blocks: Vec<CodeBlock>,
    #[serde(skip_serializing)]
    pub doc_location_links: Vec<DocLocationLink>,
    #[serde(skip_serializing)]
    pub doc_link_targets: HashMap<String, DocumentableItemId<'db>>,
    pub group: Option<String>,
    /// Path to the closest `FileLongId::OnDisk` file containing the item.
    #[serde(skip_serializing)]
    pub file_path: String,
    /// Start-offset and end-offset of the link in the file.
    #[serde(skip_serializing)]
    pub location_in_file: Option<Range<usize>>,
}

impl<'db> ItemData<'db> {
    pub fn new(
        db: &'db ScarbDocDatabase,
        id: impl TopLevelLanguageElementId<'db>,
        documentable_item_id: DocumentableItemId<'db>,
        parent_full_path: String,
    ) -> Self {
        let doc = db.get_item_documentation_as_tokens(documentable_item_id);
        let (signature, doc_location_links) =
            db.get_item_signature_with_links(documentable_item_id);
        let doc_location_links = doc_location_links
            .iter()
            .map(|link| DocLocationLink::new(link.start, link.end, link.item_id, db))
            .collect::<Vec<_>>();
        let group = find_groups_from_attributes(db, &id);
        let (file_path, span_in_file) = get_file_and_location(db, &id);
        let full_path = id.full_path(db);
        let doc = db.get_item_documentation_as_tokens(documentable_item_id);
        let code_blocks = collect_code_blocks_from_tokens(&doc, &full_path);

        Self {
            id: documentable_item_id,
            name: id.name(db).to_string(db),
            doc: doc.clone(),
            signature,
            full_path: format!("{}::{}", parent_full_path, id.name(db).long(db)),
            parent_full_path: Some(parent_full_path),
            code_blocks,
            doc_location_links,
            doc_link_targets: resolve_doc_link_targets(db, documentable_item_id, &doc),
            group,
            file_path,
            location_in_file: span_in_file,
        }
    }

    pub fn new_without_signature(
        db: &'db ScarbDocDatabase,
        id: impl TopLevelLanguageElementId<'db>,
        documentable_item_id: DocumentableItemId<'db>,
    ) -> Self {
        let doc = db.get_item_documentation_as_tokens(documentable_item_id);
        let (file_path, span_in_file) = get_file_and_location(db, &id);
        let full_path = format!(
            "{}::{}",
            doc_full_path(&id.parent_module(db), db),
            id.name(db).long(db)
        );
        let doc = db.get_item_documentation_as_tokens(documentable_item_id);
        let code_blocks = collect_code_blocks_from_tokens(&doc, &full_path);

        Self {
            id: documentable_item_id,
            name: id.name(db).to_string(db),
            doc: doc.clone(),
            signature: None,
            full_path,
            parent_full_path: Some(id.parent_module(db).full_path(db)),
            code_blocks,
            doc_location_links: vec![],
            doc_link_targets: resolve_doc_link_targets(db, documentable_item_id, &doc),
            group: find_groups_from_attributes(db, &id),
            file_path,
            location_in_file: span_in_file,
        }
    }

    pub fn new_crate(db: &'db ScarbDocDatabase, id: CrateId<'db>) -> Self {
        let documentable_id = DocumentableItemId::Crate(id);
        let doc = db.get_item_documentation_as_tokens(documentable_id);

        let module_id = ModuleId::CrateRoot(id);
        let file_path = db
            .module_main_file(module_id)
            .expect("Crate main file should always exist.")
            .full_path(db);

        let full_path = ModuleId::CrateRoot(id).full_path(db);
        let doc = db.get_item_documentation_as_tokens(documentable_id);
        let code_blocks = collect_code_blocks_from_tokens(&doc, &full_path);

        Self {
            id: documentable_id,
            name: id.long(db).name().to_string(db),
            doc: doc.clone(),
            signature: None,
            full_path,
            parent_full_path: None,
            code_blocks,
            doc_location_links: vec![],
            doc_link_targets: resolve_doc_link_targets(db, documentable_id, &doc),
            group: None,
            file_path,
            location_in_file: None,
        }
    }
}

/// Helper struct for custom serialization of [`ItemData`] for [`crate::types::other_types::Variant`] and [`crate::types::struct_types::Member`].
#[derive(Debug, Serialize, Clone)]
pub struct SubItemData<'db> {
    #[serde(skip_serializing)]
    pub id: DocumentableItemId<'db>,
    #[serde(skip_serializing)]
    pub parent_full_path: Option<String>,
    pub name: String,
    #[serde(serialize_with = "documentation_serializer")]
    pub doc: Option<Vec<DocumentationCommentToken>>,
    pub signature: Option<String>,
    pub full_path: String,
    #[serde(skip_serializing)]
    pub code_blocks: Vec<CodeBlock>,
    #[serde(skip_serializing)]
    pub doc_location_links: Vec<DocLocationLink>,
    #[serde(skip_serializing)]
    pub doc_link_targets: HashMap<String, DocumentableItemId<'db>>,
    #[serde(skip_serializing)]
    pub group: Option<String>,
    #[serde(skip_serializing)]
    pub file_path: String,
    #[serde(skip_serializing)]
    pub location_in_file: Option<Range<usize>>,
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
            code_blocks: val.code_blocks,
            doc_location_links: val.doc_location_links,
            doc_link_targets: val.doc_link_targets,
            group: val.group,
            file_path: val.file_path,
            location_in_file: val.location_in_file,
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
            doc_link_targets: val.doc_link_targets,
            group: val.group,
            file_path: val.file_path,
            location_in_file: val.location_in_file,
        }
    }
}

fn resolve_doc_link_targets<'db>(
    db: &'db ScarbDocDatabase,
    item_id: DocumentableItemId<'db>,
    doc: &Option<Vec<DocumentationCommentToken>>,
) -> HashMap<String, DocumentableItemId<'db>> {
    let mut targets = HashMap::new();
    let Some(tokens) = doc else {
        return targets;
    };

    for token in tokens {
        if let DocumentationCommentToken::Link(link) = token
            && let Some(resolved) = resolve_linked_item(db, item_id, link)
            && let Some(key) = link.md_link.dest_text.clone()
        {
            targets.entry(key.clone()).or_insert(resolved);
        }
    }

    targets
}

fn documentation_serializer<S>(
    docs: &Option<Vec<DocumentationCommentToken>>,
    serializer: S,
) -> Result<S::Ok, S::Error>
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

pub fn get_file_and_location<'db>(
    db: &'db ScarbDocDatabase,
    id: &impl TopLevelLanguageElementId<'db>,
) -> (String, Option<Range<usize>>) {
    let originating_location =
        get_originating_location(db, id.stable_location(db).span_in_file(db), None);
    let start_line = originating_location
        .span
        .start
        .position_in_file(db, originating_location.file_id)
        .map(|pos| pos.line);

    let end_line = originating_location
        .span
        .end
        .position_in_file(db, originating_location.file_id)
        .map(|pos| pos.line);

    let location = match (start_line, end_line) {
        (Some(start), Some(end)) => Some(Range { start, end }),
        _ => None,
    };

    (originating_location.file_id.full_path(db), location)
}
