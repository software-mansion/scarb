use crate::db::ScarbDocDatabase;
use crate::docs_generation::markdown::context::IncludedItems;
use crate::docs_generation::markdown::traits::WithItemDataCommon;
use crate::location_links::DocLocationLink;
use crate::types::item_data::{ItemData, SubItemData};
use crate::types::module_type::is_doc_hidden_attr;
use crate::types::other_types::doc_full_path;
use cairo_lang_defs::ids::{
    LanguageElementId, LookupItemId, MemberId, ModuleItemId, NamedLanguageElementId, StructId,
};
use cairo_lang_diagnostics::{Maybe, skip_diagnostic};
use cairo_lang_doc::db::DocGroup;
use cairo_lang_doc::documentable_item::DocumentableItemId;
use cairo_lang_doc::helpers::{
    get_generic_params, get_struct_attributes_syntax, get_syntactic_visibility,
};
use cairo_lang_doc::location_links::{LocationLink, format_signature};
use cairo_lang_doc::signature_data::SignatureDataRetriever;
use cairo_lang_doc::signature_errors::SignatureError;
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
        let mut item_data = ItemData::new_without_signature(
            db,
            id,
            LookupItemId::ModuleItem(ModuleItemId::Struct(id)).into(),
        );
        let mut signature_builder = StructSignatureBuilder::new(db, id, include_private_items)
            .map_err(|_| skip_diagnostic())?;

        let (signature, location_links, members) = signature_builder.build_signature(db);
        item_data.signature = Some(signature);
        item_data.doc_location_links = location_links;

        Ok(Self {
            id,
            node: id.stable_ptr(db),
            members,
            item_data,
        })
    }

    pub fn get_all_item_ids<'a>(&'a self) -> IncludedItems<'a, 'db> {
        self.members
            .iter()
            .map(|item| {
                (
                    item.item_data.id,
                    &item.item_data as &dyn WithItemDataCommon,
                )
            })
            .collect()
    }
}

struct MemberDataHelper<'db> {
    signature: Option<String>,
    member_id: MemberId<'db>,
    location_links: Vec<LocationLink<'db>>,
}

impl<'db> MemberDataHelper<'db> {
    fn new(member_id: MemberId<'db>, db: &'db ScarbDocDatabase) -> Self {
        let (signature, location_links) =
            db.get_item_signature_with_links(DocumentableItemId::Member(member_id));
        Self {
            signature,
            member_id,
            location_links,
        }
    }
}

struct StructSignatureBuilder<'a> {
    members_data: Vec<MemberDataHelper<'a>>,
    has_private_members: bool,
    has_public_members: bool,
    buff: String,
    location_links: Vec<LocationLink<'a>>,
}

impl<'db> StructSignatureBuilder<'db> {
    const INDENT: &'static str = "    ";
    const PRIVATE_MEMBERS: &'static str = "/* private fields */";

    fn new(
        db: &'db ScarbDocDatabase,
        id: StructId<'db>,
        include_private_items: bool,
    ) -> Result<Self, SignatureError> {
        let members = db.struct_members(id)?;
        let signature_data = StructId::retrieve_signature_data(db, id)?;

        let mut buff = String::new();
        let mut location_links = Vec::new();

        if let Some(attributes) = signature_data.attributes {
            let attributes_syntax = get_struct_attributes_syntax(attributes, db)?;
            buff.push_str(&attributes_syntax);
        }
        buff.push_str(&format!(
            "{}struct {}",
            get_syntactic_visibility(&signature_data.visibility),
            signature_data.name.long(db)
        ));

        if let Some(generic_params) = signature_data.generic_params {
            let (stx, param_location_links) = get_generic_params(generic_params, db)?;
            buff.push_str(&stx);
            location_links.extend(param_location_links);
        }
        let mut has_private_members = false;
        let mut has_public_members = false;

        let members_data: Vec<MemberDataHelper> = members
            .iter()
            .filter_map(|(_, semantic_member)| {
                let visible = matches!(semantic_member.visibility, Visibility::Public);
                let syntax_node = &semantic_member.id.stable_location(db).syntax_node(db);

                if (include_private_items || visible) && !is_doc_hidden_attr(db, syntax_node) {
                    let mdh = MemberDataHelper::new(semantic_member.id, db);
                    has_public_members = true;
                    Some(Ok(mdh))
                } else {
                    has_private_members = true;
                    None
                }
            })
            .collect::<Maybe<Vec<_>>>()?;

        Ok(StructSignatureBuilder {
            members_data,
            has_private_members,
            has_public_members,
            buff,
            location_links,
        })
    }

    fn build_signature(
        &mut self,
        db: &'db ScarbDocDatabase,
    ) -> (String, Vec<DocLocationLink>, Vec<Member<'db>>) {
        let mut members = Vec::new();
        self.buff.push_str(" {");

        for mdh in &self.members_data {
            members.push(Member::new(db, mdh.member_id));
            let mut offset = self.buff.len();
            offset += "\n".len() + Self::INDENT.len();

            let formatted_member_signature = format!(
                "\n{}{},",
                Self::INDENT,
                mdh.signature.clone().unwrap_or_default()
            );
            self.buff.push_str(&formatted_member_signature);
            self.location_links.extend(
                mdh.location_links
                    .iter()
                    .map(|link| LocationLink::new(link.start, link.end, link.item_id, offset))
                    .collect::<Vec<_>>(),
            );
        }

        if !&self.members_data.is_empty() {
            self.buff.push('\n');
        }

        if self.has_private_members {
            let (prefix, postfix) = if self.has_public_members {
                (Self::INDENT, "\n")
            } else {
                (" ", " ")
            };
            self.buff
                .push_str(&format!("{prefix}{}{postfix}", Self::PRIVATE_MEMBERS));
        }
        self.buff.push('}');

        let (new_sig_formatted, formatted_location_links) =
            format_signature(db, self.buff.clone(), self.location_links.clone());

        let doc_location_links = formatted_location_links
            .iter()
            .map(|link| DocLocationLink::new(link.start, link.end, link.item_id, db))
            .collect::<Vec<_>>();

        (new_sig_formatted, doc_location_links, members)
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
