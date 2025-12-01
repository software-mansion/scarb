use anyhow::Result;

use crate::attributes::find_groups_from_attributes;
use crate::code_blocks::{CodeBlock, collect_code_blocks_from_tokens};
use crate::db::ScarbDocDatabase;
use crate::docs_generation::markdown::context::IncludedItems;
use crate::docs_generation::markdown::traits::WithPath;
use crate::types::item_data::{ItemData, SubItemData};
use cairo_lang_defs::ids::NamedLanguageElementId;
use cairo_lang_defs::ids::{
    ConstantId, EnumId, ExternFunctionId, ExternTypeId, FreeFunctionId, ImplAliasId,
    ImplConstantDefId, ImplDefId, ImplFunctionId, ImplItemId, ImplTypeDefId, LanguageElementId,
    LookupItemId, MacroDeclarationId, ModuleId, ModuleItemId, ModuleTypeAliasId, TraitConstantId,
    TraitFunctionId, TraitId, TraitItemId, TraitTypeId, VariantId,
};
use cairo_lang_diagnostics::Maybe;
use cairo_lang_doc::documentable_item::DocumentableItemId;
use cairo_lang_semantic::items::enm::EnumSemantic;
use cairo_lang_semantic::items::imp::ImplSemantic;
use cairo_lang_semantic::items::trt::TraitSemantic;
use cairo_lang_syntax::node::ast;
use serde::Serialize;
use std::collections::HashMap;

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
    pub code_blocks: Vec<CodeBlock>,
    #[serde(skip_serializing)]
    pub doc_location_links: Vec<DocLocationLink>,
    pub group: Option<String>,
}

/// Mimics the [`TopLevelLanguageElementId::full_path`] but skips the macro modules.
/// If not omitted, the path would look like, for example,
/// `hello::define_fn_outter!(func_macro_fn_outter);::expose! {\n\t\t\tpub fn func_macro_fn_outter() -> felt252 { \n\t\t\t\tprintln!(\"hello world\");\n\t\t\t\t10 }\n\t\t}::func_macro_fn_outter`
pub fn doc_full_path(module_id: &ModuleId, db: &ScarbDocDatabase) -> String {
    match module_id {
        ModuleId::CrateRoot(id) => id.long(db).name().to_string(db),
        ModuleId::Submodule(id) => {
            format!(
                "{}::{}",
                doc_full_path(&id.parent_module(db), db),
                id.name(db).long(db)
            )
        }
        ModuleId::MacroCall { id, .. } => doc_full_path(&id.parent_module(db), db),
    }
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
        let full_path = id.full_path(db);
        let doc = db.get_item_documentation_as_tokens(documentable_item_id);
        let code_blocks = collect_code_blocks_from_tokens(&doc, &full_path);

        Self {
            id: documentable_item_id,
            name: id.name(db).to_string(db),
            doc,
            signature,
            full_path: format!("{}::{}", parent_full_path, id.name(db).long(db)),
            parent_full_path: Some(parent_full_path),
            code_blocks,
            doc_location_links,
            group,
        }
    }

    pub fn new_without_signature(
        db: &'db ScarbDocDatabase,
        id: impl TopLevelLanguageElementId<'db>,
        documentable_item_id: DocumentableItemId<'db>,
    ) -> Self {
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
            doc,
            signature: None,
            full_path,
            parent_full_path: Some(id.parent_module(db).full_path(db)),
            code_blocks,
            doc_location_links: vec![],
            group: find_groups_from_attributes(db, &id),
        }
    }

    pub fn new_crate(db: &'db ScarbDocDatabase, id: CrateId<'db>) -> Self {
        let documentable_id = DocumentableItemId::Crate(id);
        let full_path = ModuleId::CrateRoot(id).full_path(db);
        let doc = db.get_item_documentation_as_tokens(documentable_id);
        let code_blocks = collect_code_blocks_from_tokens(&doc, &full_path);

        Self {
            id: documentable_id,
            name: id.long(db).name().to_string(db),
            doc,
            signature: None,
            full_path,
            parent_full_path: None,
            code_blocks,
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
pub struct Constant<'db> {
    #[serde(skip)]
    pub id: ConstantId<'db>,
    #[serde(skip)]
    pub node: ast::ItemConstantPtr<'db>,

    pub item_data: ItemData<'db>,
}

impl<'db> Constant<'db> {
    pub fn new(db: &'db ScarbDocDatabase, id: ConstantId<'db>) -> Self {
        let node = id.stable_ptr(db);
        Self {
            id,
            node,
            item_data: ItemData::new(
                db,
                id,
                LookupItemId::ModuleItem(ModuleItemId::Constant(id)).into(),
                doc_full_path(&id.parent_module(db), db),
            ),
        }
    }
}

#[derive(Serialize, Clone)]
pub struct FreeFunction<'db> {
    #[serde(skip)]
    pub id: FreeFunctionId<'db>,
    #[serde(skip)]
    pub node: ast::FunctionWithBodyPtr<'db>,

    pub item_data: ItemData<'db>,
}

impl<'db> FreeFunction<'db> {
    pub fn new(db: &'db ScarbDocDatabase, id: FreeFunctionId<'db>) -> Self {
        let node = id.stable_ptr(db);
        let item_data = ItemData::new(
            db,
            id,
            LookupItemId::ModuleItem(ModuleItemId::FreeFunction(id)).into(),
            doc_full_path(&id.parent_module(db), db),
        );

        Self {
            id,
            node,
            item_data,
        }
    }
}

#[derive(Serialize, Clone)]
pub struct Enum<'db> {
    #[serde(skip)]
    pub id: EnumId<'db>,
    #[serde(skip)]
    pub node: ast::ItemEnumPtr<'db>,

    pub variants: Vec<Variant<'db>>,

    pub item_data: ItemData<'db>,
}

impl<'db> Enum<'db> {
    pub fn new(db: &'db ScarbDocDatabase, id: EnumId<'db>) -> Maybe<Self> {
        let variants = db.enum_variants(id)?;
        let item_data = ItemData::new(
            db,
            id,
            LookupItemId::ModuleItem(ModuleItemId::Enum(id)).into(),
            doc_full_path(&id.parent_module(db), db),
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

    pub fn get_all_item_ids<'a>(&'a self) -> IncludedItems<'a, 'db> {
        self.variants
            .iter()
            .map(|item| (item.item_data.id, &item.item_data as &dyn WithPath))
            .collect()
    }
}

#[derive(Serialize, Clone)]
pub struct Variant<'db> {
    #[serde(skip)]
    pub id: VariantId<'db>,
    #[serde(skip)]
    pub node: ast::VariantPtr<'db>,

    pub item_data: SubItemData<'db>,
}

impl<'db> Variant<'db> {
    pub fn new(db: &'db ScarbDocDatabase, id: VariantId<'db>) -> Self {
        let node = id.stable_ptr(db);
        let parent_path = format!(
            "{}::{}",
            doc_full_path(&id.parent_module(db), db),
            id.enum_id(db).name(db).to_string(db)
        );
        Self {
            id,
            node,
            item_data: ItemData::new(db, id, DocumentableItemId::Variant(id), parent_path).into(),
        }
    }
}

#[derive(Serialize, Clone)]
pub struct TypeAlias<'db> {
    #[serde(skip)]
    pub id: ModuleTypeAliasId<'db>,
    #[serde(skip)]
    pub node: ast::ItemTypeAliasPtr<'db>,

    pub item_data: ItemData<'db>,
}

impl<'db> TypeAlias<'db> {
    pub fn new(db: &'db ScarbDocDatabase, id: ModuleTypeAliasId<'db>) -> Self {
        let node = id.stable_ptr(db);
        Self {
            id,
            node,
            item_data: ItemData::new(
                db,
                id,
                LookupItemId::ModuleItem(ModuleItemId::TypeAlias(id)).into(),
                doc_full_path(&id.parent_module(db), db),
            ),
        }
    }
}

#[derive(Serialize, Clone)]
pub struct ImplAlias<'db> {
    #[serde(skip)]
    pub id: ImplAliasId<'db>,
    #[serde(skip)]
    pub node: ast::ItemImplAliasPtr<'db>,

    pub item_data: ItemData<'db>,
}

impl<'db> ImplAlias<'db> {
    pub fn new(db: &'db ScarbDocDatabase, id: ImplAliasId<'db>) -> Self {
        let node = id.stable_ptr(db);
        Self {
            id,
            node,
            item_data: ItemData::new(
                db,
                id,
                LookupItemId::ModuleItem(ModuleItemId::ImplAlias(id)).into(),
                doc_full_path(&id.parent_module(db), db),
            ),
        }
    }
}

#[derive(Serialize, Clone)]
pub struct Trait<'db> {
    #[serde(skip)]
    pub id: TraitId<'db>,
    #[serde(skip)]
    pub node: ast::ItemTraitPtr<'db>,

    pub trait_constants: Vec<TraitConstant<'db>>,
    pub trait_types: Vec<TraitType<'db>>,
    pub trait_functions: Vec<TraitFunction<'db>>,

    pub item_data: ItemData<'db>,
}

impl<'db> Trait<'db> {
    pub fn new(db: &'db ScarbDocDatabase, id: TraitId<'db>) -> Maybe<Self> {
        let item_data = ItemData::new(
            db,
            id,
            LookupItemId::ModuleItem(ModuleItemId::Trait(id)).into(),
            doc_full_path(&id.parent_module(db), db),
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

    pub fn get_all_item_ids<'a>(&'a self) -> IncludedItems<'a, 'db> {
        let mut result: IncludedItems<'a, 'db> = HashMap::default();
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
pub struct TraitConstant<'db> {
    #[serde(skip)]
    pub id: TraitConstantId<'db>,
    #[serde(skip)]
    pub node: ast::TraitItemConstantPtr<'db>,

    pub item_data: ItemData<'db>,
}

impl<'db> TraitConstant<'db> {
    pub fn new(db: &'db ScarbDocDatabase, id: TraitConstantId<'db>) -> Self {
        let node = id.stable_ptr(db);
        let parent_path = format!(
            "{}::{}",
            doc_full_path(&id.parent_module(db), db),
            id.trait_id(db).name(db).to_string(db)
        );
        Self {
            id,
            node,
            item_data: ItemData::new(
                db,
                id,
                LookupItemId::TraitItem(TraitItemId::Constant(id)).into(),
                parent_path,
            ),
        }
    }
}

#[derive(Serialize, Clone)]
pub struct TraitType<'db> {
    #[serde(skip)]
    pub id: TraitTypeId<'db>,
    #[serde(skip)]
    pub node: ast::TraitItemTypePtr<'db>,

    pub item_data: ItemData<'db>,
}

impl<'db> TraitType<'db> {
    pub fn new(db: &'db ScarbDocDatabase, id: TraitTypeId<'db>) -> Self {
        let node = id.stable_ptr(db);
        let parent_path = format!(
            "{}::{}",
            doc_full_path(&id.parent_module(db), db),
            id.trait_id(db).name(db).to_string(db)
        );
        Self {
            id,
            node,
            item_data: ItemData::new(
                db,
                id,
                LookupItemId::TraitItem(TraitItemId::Type(id)).into(),
                parent_path,
            ),
        }
    }
}

#[derive(Serialize, Clone)]
pub struct TraitFunction<'db> {
    #[serde(skip)]
    pub id: TraitFunctionId<'db>,
    #[serde(skip)]
    pub node: ast::TraitItemFunctionPtr<'db>,

    pub item_data: ItemData<'db>,
}

impl<'db> TraitFunction<'db> {
    pub fn new(db: &'db ScarbDocDatabase, id: TraitFunctionId<'db>) -> Self {
        let node = id.stable_ptr(db);
        let parent_path = format!(
            "{}::{}",
            doc_full_path(&id.parent_module(db), db),
            id.trait_id(db).name(db).to_string(db)
        );
        Self {
            id,
            node,
            item_data: ItemData::new(
                db,
                id,
                LookupItemId::TraitItem(TraitItemId::Function(id)).into(),
                parent_path,
            ),
        }
    }
}

#[derive(Serialize, Clone)]
pub struct Impl<'db> {
    #[serde(skip)]
    pub id: ImplDefId<'db>,
    #[serde(skip)]
    pub node: ast::ItemImplPtr<'db>,

    pub impl_types: Vec<ImplType<'db>>,
    pub impl_constants: Vec<ImplConstant<'db>>,
    pub impl_functions: Vec<ImplFunction<'db>>,

    pub item_data: ItemData<'db>,
}

impl<'db> Impl<'db> {
    pub fn new(db: &'db ScarbDocDatabase, id: ImplDefId<'db>) -> Maybe<Self> {
        let item_data = ItemData::new(
            db,
            id,
            LookupItemId::ModuleItem(ModuleItemId::Impl(id)).into(),
            doc_full_path(&id.parent_module(db), db),
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

    pub fn get_all_item_ids<'a>(&'a self) -> IncludedItems<'a, 'db> {
        let mut result: IncludedItems<'a, 'db> = HashMap::default();
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
pub struct ImplType<'db> {
    #[serde(skip)]
    pub id: ImplTypeDefId<'db>,
    #[serde(skip)]
    pub node: ast::ItemTypeAliasPtr<'db>,

    pub item_data: ItemData<'db>,
}

impl<'db> ImplType<'db> {
    pub fn new(db: &'db ScarbDocDatabase, id: ImplTypeDefId<'db>) -> Self {
        let node = id.stable_ptr(db);
        let parent_path = format!(
            "{}::{}",
            doc_full_path(&id.parent_module(db), db),
            id.impl_def_id(db).name(db).to_string(db)
        );
        Self {
            id,
            node,
            item_data: ItemData::new(
                db,
                id,
                LookupItemId::ImplItem(ImplItemId::Type(id)).into(),
                parent_path,
            ),
        }
    }
}

#[derive(Serialize, Clone)]
pub struct ImplConstant<'db> {
    #[serde(skip)]
    pub id: ImplConstantDefId<'db>,
    #[serde(skip)]
    pub node: ast::ItemConstantPtr<'db>,

    pub item_data: ItemData<'db>,
}

impl<'db> ImplConstant<'db> {
    pub fn new(db: &'db ScarbDocDatabase, id: ImplConstantDefId<'db>) -> Self {
        let node = id.stable_ptr(db);
        let parent_path = format!(
            "{}::{}",
            doc_full_path(&id.parent_module(db), db),
            id.impl_def_id(db).name(db).to_string(db)
        );
        Self {
            id,
            node,
            item_data: ItemData::new(
                db,
                id,
                LookupItemId::ImplItem(ImplItemId::Constant(id)).into(),
                parent_path,
            ),
        }
    }
}

#[derive(Serialize, Clone)]
pub struct ImplFunction<'db> {
    #[serde(skip)]
    pub id: ImplFunctionId<'db>,
    #[serde(skip)]
    pub node: ast::FunctionWithBodyPtr<'db>,

    pub item_data: ItemData<'db>,
}

impl<'db> ImplFunction<'db> {
    pub fn new(db: &'db ScarbDocDatabase, id: ImplFunctionId<'db>) -> Self {
        let node = id.stable_ptr(db);
        let parent_path = format!(
            "{}::{}",
            doc_full_path(&id.parent_module(db), db),
            id.impl_def_id(db).name(db).to_string(db)
        );
        Self {
            id,
            node,
            item_data: ItemData::new(
                db,
                id,
                LookupItemId::ImplItem(ImplItemId::Function(id)).into(),
                parent_path,
            ),
        }
    }
}

#[derive(Serialize, Clone)]
pub struct ExternType<'db> {
    #[serde(skip)]
    pub id: ExternTypeId<'db>,
    #[serde(skip)]
    pub node: ast::ItemExternTypePtr<'db>,

    pub item_data: ItemData<'db>,
}

impl<'db> ExternType<'db> {
    pub fn new(db: &'db ScarbDocDatabase, id: ExternTypeId<'db>) -> Self {
        let node = id.stable_ptr(db);
        Self {
            id,
            node,
            item_data: ItemData::new(
                db,
                id,
                LookupItemId::ModuleItem(ModuleItemId::ExternType(id)).into(),
                doc_full_path(&id.parent_module(db), db),
            ),
        }
    }
}

#[derive(Serialize, Clone)]
pub struct ExternFunction<'db> {
    #[serde(skip)]
    pub id: ExternFunctionId<'db>,
    #[serde(skip)]
    pub node: ast::ItemExternFunctionPtr<'db>,

    pub item_data: ItemData<'db>,
}

impl<'db> ExternFunction<'db> {
    pub fn new(db: &'db ScarbDocDatabase, id: ExternFunctionId<'db>) -> Self {
        let node = id.stable_ptr(db);
        Self {
            id,
            node,
            item_data: ItemData::new(
                db,
                id,
                LookupItemId::ModuleItem(ModuleItemId::ExternFunction(id)).into(),
                doc_full_path(&id.parent_module(db), db),
            ),
        }
    }
}

#[derive(Serialize, Clone)]
pub struct MacroDeclaration<'db> {
    #[serde(skip)]
    pub id: MacroDeclarationId<'db>,
    #[serde(skip)]
    pub node: ast::ItemMacroDeclarationPtr<'db>,
    pub item_data: ItemData<'db>,
}

impl<'db> MacroDeclaration<'db> {
    pub fn new(db: &'db ScarbDocDatabase, id: MacroDeclarationId<'db>) -> Self {
        let node = id.stable_ptr(db);
        Self {
            id,
            node,
            item_data: ItemData::new(
                db,
                id,
                LookupItemId::ModuleItem(ModuleItemId::MacroDeclaration(id)).into(),
                doc_full_path(&id.parent_module(db), db),
            ),
        }
    }
}
