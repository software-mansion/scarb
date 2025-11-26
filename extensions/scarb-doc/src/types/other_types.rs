use crate::db::ScarbDocDatabase;
use crate::docs_generation::markdown::context::IncludedItems;
use crate::docs_generation::markdown::traits::WithPath;
use crate::types::item_data::{ItemData, SubItemData};
use crate::types::module_type::is_doc_hidden_attr;
use cairo_lang_defs::ids::{
    ConstantId, EnumId, ExternFunctionId, ExternTypeId, FreeFunctionId, ImplAliasId,
    ImplConstantDefId, ImplDefId, ImplFunctionId, ImplItemId, ImplTypeDefId, LanguageElementId,
    LookupItemId, MacroDeclarationId, MemberId, ModuleId, ModuleItemId, ModuleTypeAliasId,
    NamedLanguageElementId, StructId, TraitConstantId, TraitFunctionId, TraitId, TraitItemId,
    TraitTypeId, VariantId,
};
use cairo_lang_diagnostics::Maybe;
use cairo_lang_doc::documentable_item::DocumentableItemId;
use cairo_lang_semantic::items::enm::EnumSemantic;
use cairo_lang_semantic::items::imp::ImplSemantic;
use cairo_lang_semantic::items::structure::StructSemantic;
use cairo_lang_semantic::items::trt::TraitSemantic;
use cairo_lang_semantic::items::visibility::Visibility;
use cairo_lang_syntax::node::ast;
use serde::Serialize;
use std::collections::HashMap;

/// Mimics the [`cairo_lang_defs::ids::TopLevelLanguageElementId::full_path`] but skips the macro modules.
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
