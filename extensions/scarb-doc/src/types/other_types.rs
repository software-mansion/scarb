use anyhow::Result;

use crate::attributes::find_groups_from_attributes;
use crate::db::ScarbDocDatabase;
use crate::location_links::DocLocationLink;
use crate::types::module_type::is_doc_hidden_attr;
use cairo_lang_defs::ids::{
    ConstantId, EnumId, ExternFunctionId, ExternTypeId, FreeFunctionId, ImplAliasId,
    ImplConstantDefId, ImplDefId, ImplFunctionId, ImplItemId, ImplTypeDefId, LanguageElementId,
    LookupItemId, MemberId, ModuleId, ModuleItemId, ModuleTypeAliasId, StructId,
    TopLevelLanguageElementId, TraitConstantId, TraitFunctionId, TraitId, TraitItemId, TraitTypeId,
    VariantId,
};
use cairo_lang_diagnostics::Maybe;
use cairo_lang_doc::db::DocGroup;
use cairo_lang_doc::documentable_item::DocumentableItemId;
use cairo_lang_doc::parser::DocumentationCommentToken;
use cairo_lang_filesystem::ids::CrateId;
use cairo_lang_semantic::db::SemanticGroup;
use cairo_lang_semantic::items::visibility::Visibility;
use cairo_lang_syntax::node::ast;
use serde::Serialize;
use serde::Serializer;
use std::collections::HashMap;
use std::fmt::Debug;

#[derive(Debug, Serialize, Clone)]
pub struct ItemData {
    #[serde(skip_serializing)]
    pub id: DocumentableItemId,
    #[serde(skip_serializing)]
    pub parent_full_path: Option<String>,
    pub name: String,
    #[serde(serialize_with = "documentation_serializer")]
    pub doc: Option<Vec<DocumentationCommentToken>>,
    pub signature: Option<String>,
    pub full_path: String,
    #[serde(skip_serializing)]
    pub doc_location_links: Vec<DocLocationLink>,
    pub group: Option<String>,
}

impl ItemData {
    pub fn new(
        db: &ScarbDocDatabase,
        id: impl TopLevelLanguageElementId,
        documentable_item_id: DocumentableItemId,
    ) -> Self {
        let (signature, doc_location_links) =
            db.get_item_signature_with_links(documentable_item_id);
        let doc_location_links = doc_location_links
            .iter()
            .filter(|link| {
                if let Some(stable_location) = link.item_id.stable_location(db) {
                    !is_doc_hidden_attr(db, &stable_location.syntax_node(db))
                } else {
                    false
                }
            })
            .map(|link| DocLocationLink::new(link.start, link.end, link.item_id, db))
            .collect::<Vec<_>>();
        let group = find_groups_from_attributes(db, &id);
        Self {
            id: documentable_item_id,
            name: id.name(db).into(),
            doc: db.get_item_documentation_as_tokens(documentable_item_id),
            signature,
            full_path: id.full_path(db),
            parent_full_path: Some(id.parent_module(db).full_path(db)),
            doc_location_links,
            group,
        }
    }

    pub fn new_without_signature(
        db: &ScarbDocDatabase,
        id: impl TopLevelLanguageElementId,
        documentable_item_id: DocumentableItemId,
    ) -> Self {
        Self {
            id: documentable_item_id,
            name: id.name(db).into(),
            doc: db.get_item_documentation_as_tokens(documentable_item_id),
            signature: None,
            full_path: id.full_path(db),
            parent_full_path: Some(id.parent_module(db).full_path(db)),
            doc_location_links: vec![],
            group: find_groups_from_attributes(db, &id),
        }
    }

    pub fn new_crate(db: &ScarbDocDatabase, id: CrateId) -> Self {
        let documentable_id = DocumentableItemId::Crate(id);
        Self {
            id: documentable_id,
            name: id.name(db).into(),
            doc: db.get_item_documentation_as_tokens(documentable_id),
            signature: None,
            full_path: ModuleId::CrateRoot(id).full_path(db),
            parent_full_path: None,
            doc_location_links: vec![],
            group: None,
        }
    }
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

#[derive(Serialize, Clone)]
pub struct Constant {
    #[serde(skip)]
    pub id: ConstantId,
    #[serde(skip)]
    pub node: ast::ItemConstantPtr,

    pub item_data: ItemData,
}

impl Constant {
    pub fn new(db: &ScarbDocDatabase, id: ConstantId) -> Self {
        let node = id.stable_ptr(db);
        Self {
            id,
            node,
            item_data: ItemData::new(
                db,
                id,
                LookupItemId::ModuleItem(ModuleItemId::Constant(id)).into(),
            ),
        }
    }
}

#[derive(Serialize, Clone)]
pub struct FreeFunction {
    #[serde(skip)]
    pub id: FreeFunctionId,
    #[serde(skip)]
    pub node: ast::FunctionWithBodyPtr,

    pub item_data: ItemData,
}

impl FreeFunction {
    pub fn new(db: &ScarbDocDatabase, id: FreeFunctionId) -> Self {
        let node = id.stable_ptr(db);
        Self {
            id,
            node,
            item_data: ItemData::new(
                db,
                id,
                LookupItemId::ModuleItem(ModuleItemId::FreeFunction(id)).into(),
            ),
        }
    }
}

#[derive(Serialize, Clone)]
pub struct Struct {
    #[serde(skip)]
    pub id: StructId,
    #[serde(skip)]
    pub node: ast::ItemStructPtr,

    pub members: Vec<Member>,

    pub item_data: ItemData,
}

impl Struct {
    pub fn new(db: &ScarbDocDatabase, id: StructId, include_private_items: bool) -> Maybe<Self> {
        let members = db.struct_members(id)?;

        let item_data = ItemData::new(
            db,
            id,
            LookupItemId::ModuleItem(ModuleItemId::Struct(id)).into(),
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

    pub fn get_all_item_ids(&self) -> HashMap<DocumentableItemId, &ItemData> {
        self.members
            .iter()
            .map(|item| (item.item_data.id, &item.item_data))
            .collect()
    }
}

#[derive(Serialize, Clone)]
pub struct Member {
    #[serde(skip)]
    pub id: MemberId,
    #[serde(skip)]
    pub node: ast::MemberPtr,

    pub item_data: ItemData,
}

impl Member {
    pub fn new(db: &ScarbDocDatabase, id: MemberId) -> Self {
        let node = id.stable_ptr(db);

        Self {
            id,
            node,
            item_data: ItemData::new(db, id, DocumentableItemId::Member(id)),
        }
    }
}

#[derive(Serialize, Clone)]
pub struct Enum {
    #[serde(skip)]
    pub id: EnumId,
    #[serde(skip)]
    pub node: ast::ItemEnumPtr,

    pub variants: Vec<Variant>,

    pub item_data: ItemData,
}

impl Enum {
    pub fn new(db: &ScarbDocDatabase, id: EnumId) -> Maybe<Self> {
        let variants = db.enum_variants(id)?;
        let item_data = ItemData::new(
            db,
            id,
            LookupItemId::ModuleItem(ModuleItemId::Enum(id)).into(),
        );

        let variants = variants
            .iter()
            .map(|(_name, variant_id)| Variant::new(db, *variant_id))
            .collect::<Vec<_>>();

        let node = id.stable_ptr(db);
        Ok(Self {
            id,
            node,
            variants,
            item_data,
        })
    }

    pub fn get_all_item_ids(&self) -> HashMap<DocumentableItemId, &ItemData> {
        self.variants
            .iter()
            .map(|item| (item.item_data.id, &item.item_data))
            .collect()
    }
}

#[derive(Serialize, Clone)]
pub struct Variant {
    #[serde(skip)]
    pub id: VariantId,
    #[serde(skip)]
    pub node: ast::VariantPtr,

    pub item_data: ItemData,
}

impl Variant {
    pub fn new(db: &ScarbDocDatabase, id: VariantId) -> Self {
        let node = id.stable_ptr(db);

        Self {
            id,
            node,
            item_data: ItemData::new(db, id, DocumentableItemId::Variant(id)),
        }
    }
}

#[derive(Serialize, Clone)]
pub struct TypeAlias {
    #[serde(skip)]
    pub id: ModuleTypeAliasId,
    #[serde(skip)]
    pub node: ast::ItemTypeAliasPtr,

    pub item_data: ItemData,
}

impl TypeAlias {
    pub fn new(db: &ScarbDocDatabase, id: ModuleTypeAliasId) -> Self {
        let node = id.stable_ptr(db);
        Self {
            id,
            node,
            item_data: ItemData::new(
                db,
                id,
                LookupItemId::ModuleItem(ModuleItemId::TypeAlias(id)).into(),
            ),
        }
    }
}

#[derive(Serialize, Clone)]
pub struct ImplAlias {
    #[serde(skip)]
    pub id: ImplAliasId,
    #[serde(skip)]
    pub node: ast::ItemImplAliasPtr,

    pub item_data: ItemData,
}

impl ImplAlias {
    pub fn new(db: &ScarbDocDatabase, id: ImplAliasId) -> Self {
        let node = id.stable_ptr(db);
        Self {
            id,
            node,
            item_data: ItemData::new(
                db,
                id,
                LookupItemId::ModuleItem(ModuleItemId::ImplAlias(id)).into(),
            ),
        }
    }
}

#[derive(Serialize, Clone)]
pub struct Trait {
    #[serde(skip)]
    pub id: TraitId,
    #[serde(skip)]
    pub node: ast::ItemTraitPtr,

    pub trait_constants: Vec<TraitConstant>,
    pub trait_types: Vec<TraitType>,
    pub trait_functions: Vec<TraitFunction>,

    pub item_data: ItemData,
}

impl Trait {
    pub fn new(db: &ScarbDocDatabase, id: TraitId) -> Maybe<Self> {
        let item_data = ItemData::new(
            db,
            id,
            LookupItemId::ModuleItem(ModuleItemId::Trait(id)).into(),
        );

        let trait_constants = db.trait_constants(id)?;
        let trait_constants = trait_constants
            .iter()
            .map(|(_name, trait_constant_id)| TraitConstant::new(db, *trait_constant_id))
            .collect::<Vec<_>>();

        let trait_types = db.trait_types(id)?;
        let trait_types = trait_types
            .iter()
            .map(|(_name, trait_type_id)| TraitType::new(db, *trait_type_id))
            .collect::<Vec<_>>();

        let trait_functions = db.trait_functions(id)?;
        let trait_functions = trait_functions
            .iter()
            .map(|(_name, trait_function_id)| TraitFunction::new(db, *trait_function_id))
            .collect::<Vec<_>>();

        let node = id.stable_ptr(db);
        Ok(Self {
            id,
            node,
            trait_constants,
            trait_types,
            trait_functions,
            item_data,
        })
    }

    pub fn get_all_item_ids(&self) -> HashMap<DocumentableItemId, &ItemData> {
        let mut result: HashMap<DocumentableItemId, &ItemData> = HashMap::default();
        self.trait_constants.iter().for_each(|item| {
            result.insert(item.item_data.id, &item.item_data);
        });
        self.trait_functions.iter().for_each(|item| {
            result.insert(item.item_data.id, &item.item_data);
        });
        self.trait_types.iter().for_each(|item| {
            result.insert(item.item_data.id, &item.item_data);
        });
        result
    }
}

#[derive(Serialize, Clone)]
pub struct TraitConstant {
    #[serde(skip)]
    pub id: TraitConstantId,
    #[serde(skip)]
    pub node: ast::TraitItemConstantPtr,

    pub item_data: ItemData,
}

impl TraitConstant {
    pub fn new(db: &ScarbDocDatabase, id: TraitConstantId) -> Self {
        let node = id.stable_ptr(db);

        Self {
            id,
            node,
            item_data: ItemData::new(
                db,
                id,
                LookupItemId::TraitItem(TraitItemId::Constant(id)).into(),
            ),
        }
    }
}

#[derive(Serialize, Clone)]
pub struct TraitType {
    #[serde(skip)]
    pub id: TraitTypeId,
    #[serde(skip)]
    pub node: ast::TraitItemTypePtr,

    pub item_data: ItemData,
}

impl TraitType {
    pub fn new(db: &ScarbDocDatabase, id: TraitTypeId) -> Self {
        let node = id.stable_ptr(db);

        Self {
            id,
            node,
            item_data: ItemData::new(
                db,
                id,
                LookupItemId::TraitItem(TraitItemId::Type(id)).into(),
            ),
        }
    }
}

#[derive(Serialize, Clone)]
pub struct TraitFunction {
    #[serde(skip)]
    pub id: TraitFunctionId,
    #[serde(skip)]
    pub node: ast::TraitItemFunctionPtr,

    pub item_data: ItemData,
}

impl TraitFunction {
    pub fn new(db: &ScarbDocDatabase, id: TraitFunctionId) -> Self {
        let node = id.stable_ptr(db);

        Self {
            id,
            node,
            item_data: ItemData::new(
                db,
                id,
                LookupItemId::TraitItem(TraitItemId::Function(id)).into(),
            ),
        }
    }
}

#[derive(Serialize, Clone)]
pub struct Impl {
    #[serde(skip)]
    pub id: ImplDefId,
    #[serde(skip)]
    pub node: ast::ItemImplPtr,

    pub impl_types: Vec<ImplType>,
    pub impl_constants: Vec<ImplConstant>,
    pub impl_functions: Vec<ImplFunction>,

    pub item_data: ItemData,
}

impl Impl {
    pub fn new(db: &ScarbDocDatabase, id: ImplDefId) -> Maybe<Self> {
        let item_data = ItemData::new(
            db,
            id,
            LookupItemId::ModuleItem(ModuleItemId::Impl(id)).into(),
        );

        let impl_types = db.impl_types(id)?;
        let impl_types = impl_types
            .iter()
            .map(|(id, _)| ImplType::new(db, *id))
            .collect::<Vec<_>>();

        let impl_constants = db.impl_constants(id)?;
        let impl_constants = impl_constants
            .iter()
            .map(|(id, _)| ImplConstant::new(db, *id))
            .collect::<Vec<_>>();

        let impl_functions = db.impl_functions(id)?;
        let impl_functions = impl_functions
            .iter()
            .map(|(_name, id)| ImplFunction::new(db, *id))
            .collect::<Vec<_>>();

        let node = id.stable_ptr(db);
        Ok(Self {
            id,
            node,
            impl_types,
            impl_constants,
            impl_functions,
            item_data,
        })
    }

    pub fn get_all_item_ids(&self) -> HashMap<DocumentableItemId, &ItemData> {
        let mut result: HashMap<DocumentableItemId, &ItemData> = HashMap::default();
        self.impl_constants.iter().for_each(|item| {
            result.insert(item.item_data.id, &item.item_data);
        });
        self.impl_functions.iter().for_each(|item| {
            result.insert(item.item_data.id, &item.item_data);
        });
        self.impl_types.iter().for_each(|item| {
            result.insert(item.item_data.id, &item.item_data);
        });
        result
    }
}

#[derive(Serialize, Clone)]
pub struct ImplType {
    #[serde(skip)]
    pub id: ImplTypeDefId,
    #[serde(skip)]
    pub node: ast::ItemTypeAliasPtr,

    pub item_data: ItemData,
}

impl ImplType {
    pub fn new(db: &ScarbDocDatabase, id: ImplTypeDefId) -> Self {
        let node = id.stable_ptr(db);

        Self {
            id,
            node,
            item_data: ItemData::new(db, id, LookupItemId::ImplItem(ImplItemId::Type(id)).into()),
        }
    }
}

#[derive(Serialize, Clone)]
pub struct ImplConstant {
    #[serde(skip)]
    pub id: ImplConstantDefId,
    #[serde(skip)]
    pub node: ast::ItemConstantPtr,

    pub item_data: ItemData,
}

impl ImplConstant {
    pub fn new(db: &ScarbDocDatabase, id: ImplConstantDefId) -> Self {
        let node = id.stable_ptr(db);

        Self {
            id,
            node,
            item_data: ItemData::new(
                db,
                id,
                LookupItemId::ImplItem(ImplItemId::Constant(id)).into(),
            ),
        }
    }
}

#[derive(Serialize, Clone)]
pub struct ImplFunction {
    #[serde(skip)]
    pub id: ImplFunctionId,
    #[serde(skip)]
    pub node: ast::FunctionWithBodyPtr,

    pub item_data: ItemData,
}

impl ImplFunction {
    pub fn new(db: &ScarbDocDatabase, id: ImplFunctionId) -> Self {
        let node = id.stable_ptr(db);

        Self {
            id,
            node,
            item_data: ItemData::new(
                db,
                id,
                LookupItemId::ImplItem(ImplItemId::Function(id)).into(),
            ),
        }
    }
}

#[derive(Serialize, Clone)]
pub struct ExternType {
    #[serde(skip)]
    pub id: ExternTypeId,
    #[serde(skip)]
    pub node: ast::ItemExternTypePtr,

    pub item_data: ItemData,
}

impl ExternType {
    pub fn new(db: &ScarbDocDatabase, id: ExternTypeId) -> Self {
        let node = id.stable_ptr(db);
        Self {
            id,
            node,
            item_data: ItemData::new(
                db,
                id,
                LookupItemId::ModuleItem(ModuleItemId::ExternType(id)).into(),
            ),
        }
    }
}

#[derive(Serialize, Clone)]
pub struct ExternFunction {
    #[serde(skip)]
    pub id: ExternFunctionId,
    #[serde(skip)]
    pub node: ast::ItemExternFunctionPtr,

    pub item_data: ItemData,
}

impl ExternFunction {
    pub fn new(db: &ScarbDocDatabase, id: ExternFunctionId) -> Self {
        let node = id.stable_ptr(db);
        Self {
            id,
            node,
            item_data: ItemData::new(
                db,
                id,
                LookupItemId::ModuleItem(ModuleItemId::ExternFunction(id)).into(),
            ),
        }
    }
}
