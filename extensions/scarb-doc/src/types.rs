use cairo_lang_semantic::items::functions::GenericFunctionId;
use cairo_lang_semantic::items::us::SemanticUseEx;
use cairo_lang_semantic::items::visibility;
use cairo_lang_semantic::resolve::ResolvedGenericItem;
use cairo_lang_utils::Upcast;
use itertools::{chain, Itertools};
use serde::Serialize;

use cairo_lang_defs::db::DefsGroup;
use cairo_lang_defs::diagnostic_utils::StableLocation;
use cairo_lang_defs::ids::{
    ConstantId, EnumId, ExternFunctionId, ExternTypeId, FreeFunctionId, GenericTypeId, ImplAliasId,
    ImplConstantDefId, ImplDefId, ImplFunctionId, ImplItemId, ImplTypeDefId, LookupItemId,
    MemberId, ModuleId, ModuleItemId, ModuleTypeAliasId, StructId, SubmoduleId,
    TopLevelLanguageElementId, TraitConstantId, TraitFunctionId, TraitId, TraitItemId, TraitTypeId,
    UseId, VariantId,
};
use cairo_lang_doc::db::DocGroup;
use cairo_lang_doc::documentable_item::DocumentableItemId;
use cairo_lang_filesystem::ids::CrateId;
use cairo_lang_semantic::db::SemanticGroup;
use cairo_lang_syntax::node::ast;

use crate::db::ScarbDocDatabase;

#[derive(Serialize, Clone)]
pub struct Crate {
    pub root_module: Module,
}

impl Crate {
    pub fn new(db: &ScarbDocDatabase, crate_id: CrateId, include_private_items: bool) -> Self {
        let root_module_id = ModuleId::CrateRoot(crate_id);
        Self {
            root_module: Module::new(db, root_module_id, root_module_id, include_private_items),
        }
    }
}

fn is_visible_in_module(
    db: &ScarbDocDatabase,
    root_module_id: ModuleId,
    element_id: &dyn TopLevelLanguageElementId,
) -> bool {
    let cotaining_module_id = element_id.parent_module(db);
    match db
        .module_item_info_by_name(cotaining_module_id, element_id.name(db.upcast()))
        .unwrap()
    {
        Some(module_item_info) => visibility::peek_visible_in(
            db,
            module_item_info.visibility,
            cotaining_module_id,
            root_module_id,
        ),
        None => false,
    }
}

#[derive(Serialize, Clone)]
pub struct Module {
    #[serde(skip)]
    pub module_id: ModuleId,
    pub item_data: ItemData,

    pub submodules: Vec<Module>,
    pub constants: Vec<Constant>,
    pub free_functions: Vec<FreeFunction>,
    pub structs: Vec<Struct>,
    pub enums: Vec<Enum>,
    pub type_aliases: Vec<TypeAlias>,
    pub impl_aliases: Vec<ImplAlias>,
    pub traits: Vec<Trait>,
    pub impls: Vec<Impl>,
    pub extern_types: Vec<ExternType>,
    pub extern_functions: Vec<ExternFunction>,
}

impl Module {
    pub fn new(
        db: &ScarbDocDatabase,
        root_module_id: ModuleId,
        module_id: ModuleId,
        include_private_items: bool,
    ) -> Self {
        let item_data = match module_id {
            ModuleId::CrateRoot(crate_id) => ItemData::new_crate(db, crate_id),
            ModuleId::Submodule(submodule_id) => ItemData::new_without_signature(
                db,
                submodule_id,
                LookupItemId::ModuleItem(ModuleItemId::Submodule(submodule_id)).into(),
            ),
        };

        let should_include_item = |id: &dyn TopLevelLanguageElementId| {
            if include_private_items {
                return true;
            }
            is_visible_in_module(db, root_module_id, id)
        };

        let module_uses_items: Vec<ResolvedGenericItem> = db
            .module_uses(module_id)
            .unwrap()
            .iter()
            .filter_map(|(use_id, _)| {
                if should_include_item(use_id) {
                    Some(db.use_resolved_item(*use_id).unwrap())
                } else {
                    None
                }
            })
            .collect();

        let (
            use_constants,
            use_free_functions,
            use_structs,
            use_enums,
            use_module_type_aliases,
            use_impl_aliases,
            use_traits,
            use_impl_defs,
            use_extern_types,
            use_extern_functions,
            use_submodules,
            use_crates,
        ) = extract_items_from_module_uses(module_uses_items);

        println!(
            "consts: {:?}\nfree funcs:{:?}\nstructs{:?}\nenums:{:?}\n{:?}\n{:?}\n{:?}\n{:?}\n{:?}\n{:?}\n{:?}\n{:?}\n",
            use_constants,
            use_free_functions,
            use_structs,
            use_enums,
            use_module_type_aliases,
            use_impl_aliases,
            use_traits,
            use_impl_defs,
            use_extern_types,
            use_extern_functions,
            use_submodules,
            use_crates
        );

        let module_constants = db.module_constants(module_id).unwrap();
        let constants = chain!(module_constants.keys(), use_constants.iter())
            .filter(|id| should_include_item(*id))
            .map(|id| Constant::new(db, *id))
            .collect();

        let module_free_functions = db.module_free_functions(module_id).unwrap();
        let free_functions = chain!(module_free_functions.keys(), use_free_functions.iter())
            .filter(|id| should_include_item(*id))
            .map(|id| FreeFunction::new(db, *id))
            .collect();

        let module_structs = db.module_structs(module_id).unwrap();
        let structs = chain!(module_structs.keys(), use_structs.iter())
            .filter(|id| should_include_item(*id))
            .map(|id| Struct::new(db, *id, root_module_id, include_private_items))
            .collect();

        let module_enums = db.module_enums(module_id).unwrap();
        let enums = chain!(module_enums.keys(), use_enums.iter())
            .filter(|id| should_include_item(*id))
            .map(|id| Enum::new(db, *id))
            .collect();

        let module_type_aliases = db.module_type_aliases(module_id).unwrap();
        let type_aliases = chain!(module_type_aliases.keys(), use_module_type_aliases.iter())
            .filter(|id| should_include_item(*id))
            .map(|id| TypeAlias::new(db, *id))
            .collect();

        let module_impl_aliases = db.module_impl_aliases(module_id).unwrap();
        let impl_aliases = chain!(module_impl_aliases.keys(), use_impl_aliases.iter())
            .filter(|id| should_include_item(*id))
            .map(|id| ImplAlias::new(db, *id))
            .collect();

        let module_traits = db.module_traits(module_id).unwrap();
        let traits = chain!(module_traits.keys(), use_traits.iter())
            .filter(|id| should_include_item(*id))
            .map(|id| Trait::new(db, *id))
            .collect();

        let module_impls = db.module_impls(module_id).unwrap();
        let impls = chain!(module_impls.keys(), use_impl_defs.iter())
            .filter(|id| should_include_item(*id))
            .map(|id| Impl::new(db, *id))
            .collect();

        let module_extern_types = db.module_extern_types(module_id).unwrap();
        let extern_types = chain!(module_extern_types.keys(), use_extern_types.iter())
            .filter(|id| should_include_item(*id))
            .map(|id| ExternType::new(db, *id))
            .collect();

        let module_extern_functions = db.module_extern_functions(module_id).unwrap();
        let extern_functions = chain!(module_extern_functions.keys(), use_extern_functions.iter())
            .filter(|id| should_include_item(*id))
            .map(|id| ExternFunction::new(db, *id))
            .collect();

        let module_uses = db.module_uses(module_id).unwrap();
        module_uses
            .iter()
            .filter(|(use_id, _)| should_include_item(*use_id))
            .for_each(|(use_id, _)| {
                let used_item = db.use_resolved_item(*use_id).unwrap();
                match used_item {
                    ResolvedGenericItem::Module(ModuleId::CrateRoot(module_id)) => {
                        println!("crate {:?}", module_id.name(db.upcast()))
                    }
                    ResolvedGenericItem::Module(ModuleId::Submodule(id)) => {
                        println!("submodule {:?}", ModuleId::Submodule(id).name(db.upcast()))
                    }
                    _ => (),
                }
            });

        // module_uses
        //     .iter()
        //     .filter(|(use_id, _)| should_include_item(*use_id))
        //     .for_each(|(use_id, _)| {
        //         let used_item = db.use_resolved_item(*use_id).unwrap();
        //         match used_item {
        //             ResolvedGenericItem::Module(ModuleId::Submodule(module_id)) => {
        //                 println!("submodule");
        //             }
        //             _ => (),
        //         }
        //     });

        // let resolved_modules_from_uses = module_uses.iter().filter_map(|(use_id, _)| {
        //     if !should_include_item(use_id) {
        //         return None;
        //     }
        //     let item = db.use_resolved_item(*use_id).unwrap();
        //     match item {
        //         ResolvedGenericItem::Module(module_id) => Some(module_id),
        //         _ => None,
        //     }
        // });
        // .filter(|id| should_include_item(*id))
        // .map(|id| {
        //     Self::new(
        //         db,
        //         root_module_id,
        //         ModuleId::Submodule(*id),
        //         include_private_items,
        //     )
        // })
        // .collect();

        let module_submodules = db.module_submodules(module_id).unwrap();
        let mut submodules: Vec<Module> = chain!(module_submodules.keys(), use_submodules.iter())
            .filter(|id| should_include_item(*id))
            .map(|id| {
                Self::new(
                    db,
                    root_module_id,
                    ModuleId::Submodule(*id),
                    include_private_items,
                )
            })
            .collect();

        let reexported_crates_as_modules: Vec<Module> = use_crates
            .iter()
            .map(|id| {
                Self::new(
                    db,
                    root_module_id,
                    ModuleId::CrateRoot(*id),
                    include_private_items,
                )
            })
            .collect();

        submodules.extend(reexported_crates_as_modules);

        Self {
            module_id,
            item_data,
            submodules,
            constants,
            free_functions,
            structs,
            enums,
            type_aliases,
            impl_aliases,
            traits,
            impls,
            extern_types,
            extern_functions,
        }
    }
}

#[derive(Serialize, Clone)]
pub struct ItemData {
    pub name: String,
    pub doc: Option<String>,
    pub signature: Option<String>,
    pub full_path: String,
}

impl ItemData {
    pub fn new(
        db: &ScarbDocDatabase,
        id: impl TopLevelLanguageElementId,
        documentable_item_id: DocumentableItemId,
    ) -> Self {
        Self {
            name: id.name(db).into(),
            doc: db.get_item_documentation(documentable_item_id),
            signature: Some(db.get_item_signature(documentable_item_id)),
            full_path: id.full_path(db),
        }
    }

    pub fn new_without_signature(
        db: &ScarbDocDatabase,
        id: impl TopLevelLanguageElementId,
        documentable_item_id: DocumentableItemId,
    ) -> Self {
        Self {
            name: id.name(db).into(),
            doc: db.get_item_documentation(documentable_item_id),
            signature: None,
            full_path: id.full_path(db),
        }
    }

    pub fn new_crate(db: &ScarbDocDatabase, id: CrateId) -> Self {
        Self {
            name: id.name(db).into(),
            doc: db.get_item_documentation(DocumentableItemId::Crate(id)),
            signature: None,
            full_path: ModuleId::CrateRoot(id).full_path(db),
        }
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
    pub fn new(
        db: &ScarbDocDatabase,
        id: StructId,
        root_module_id: ModuleId,
        include_private_items: bool,
    ) -> Self {
        let members = db.struct_members(id).unwrap();

        let item_data = ItemData::new_without_signature(
            db,
            id,
            LookupItemId::ModuleItem(ModuleItemId::Struct(id)).into(),
        );

        let members = members
            .iter()
            .filter(|(_, semantic_member)| {
                include_private_items
                    || is_visible_in_module(db, root_module_id, &semantic_member.id)
            })
            .map(|(_name, semantic_member)| Member::new(db, semantic_member.id))
            .collect::<Vec<_>>();

        let node = id.stable_ptr(db);
        Self {
            id,
            node,
            members,
            item_data,
        }
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
    pub fn new(db: &ScarbDocDatabase, id: EnumId) -> Self {
        let variants = db.enum_variants(id).unwrap();
        let item_data = ItemData::new_without_signature(
            db,
            id,
            LookupItemId::ModuleItem(ModuleItemId::Enum(id)).into(),
        );

        let variants = variants
            .iter()
            .map(|(_name, variant_id)| Variant::new(db, *variant_id))
            .collect::<Vec<_>>();

        let node = id.stable_ptr(db);
        Self {
            id,
            node,
            variants,
            item_data,
        }
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
    pub fn new(db: &ScarbDocDatabase, id: TraitId) -> Self {
        let item_data = ItemData::new(
            db,
            id,
            LookupItemId::ModuleItem(ModuleItemId::Trait(id)).into(),
        );

        let trait_constants = db.trait_constants(id).unwrap();
        let trait_constants = trait_constants
            .iter()
            .map(|(_name, trait_constant_id)| TraitConstant::new(db, *trait_constant_id))
            .collect::<Vec<_>>();

        let trait_types = db.trait_types(id).unwrap();
        let trait_types = trait_types
            .iter()
            .map(|(_name, trait_type_id)| TraitType::new(db, *trait_type_id))
            .collect::<Vec<_>>();

        let trait_functions = db.trait_functions(id).unwrap();
        let trait_functions = trait_functions
            .iter()
            .map(|(_name, trait_function_id)| TraitFunction::new(db, *trait_function_id))
            .collect::<Vec<_>>();

        let node = id.stable_ptr(db);
        Self {
            id,
            node,
            trait_constants,
            trait_types,
            trait_functions,
            item_data,
        }
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
    pub fn new(db: &ScarbDocDatabase, id: ImplDefId) -> Self {
        let item_data = ItemData::new(
            db,
            id,
            LookupItemId::ModuleItem(ModuleItemId::Impl(id)).into(),
        );

        let impl_types = db.impl_types(id).unwrap();
        let impl_types = impl_types
            .iter()
            .map(|(id, _)| ImplType::new(db, *id))
            .collect::<Vec<_>>();

        let impl_constants = db.impl_constants(id).unwrap();
        let impl_constants = impl_constants
            .iter()
            .map(|(id, _)| ImplConstant::new(db, *id))
            .collect::<Vec<_>>();

        let impl_functions = db.impl_functions(id).unwrap();
        let impl_functions = impl_functions
            .iter()
            .map(|(_name, id)| ImplFunction::new(db, *id))
            .collect::<Vec<_>>();

        let node = id.stable_ptr(db);
        Self {
            id,
            node,
            impl_types,
            impl_constants,
            impl_functions,
            item_data,
        }
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

fn extract_items_from_module_uses(
    module_use_items: Vec<ResolvedGenericItem>,
) -> (
    Vec<ConstantId>,
    Vec<FreeFunctionId>,
    Vec<StructId>,
    Vec<EnumId>,
    Vec<ModuleTypeAliasId>,
    Vec<ImplAliasId>,
    Vec<TraitId>,
    Vec<ImplDefId>,
    Vec<ExternTypeId>,
    Vec<ExternFunctionId>,
    Vec<SubmoduleId>,
    Vec<CrateId>,
) {
    let mut constants = Vec::new();
    let mut free_functions = Vec::new();
    let mut structs = Vec::new();
    let mut enums = Vec::new();
    let mut module_type_aliases = Vec::new();
    let mut impl_aliases = Vec::new();
    let mut traits = Vec::new();
    let mut impl_defs = Vec::new();
    let mut extern_types = Vec::new();
    let mut extern_functions = Vec::new();
    let mut submodules = Vec::new();
    let mut crates = Vec::new();

    for item in module_use_items {
        match item {
            ResolvedGenericItem::GenericConstant(id) => constants.push(id),
            ResolvedGenericItem::GenericFunction(GenericFunctionId::Free(id)) => {
                free_functions.push(id)
            }
            ResolvedGenericItem::GenericType(GenericTypeId::Struct(id)) => structs.push(id),
            ResolvedGenericItem::GenericType(GenericTypeId::Enum(id)) => enums.push(id),
            ResolvedGenericItem::GenericTypeAlias(id) => module_type_aliases.push(id),
            ResolvedGenericItem::GenericImplAlias(id) => impl_aliases.push(id),
            ResolvedGenericItem::Trait(id) => traits.push(id),
            ResolvedGenericItem::Impl(id) => impl_defs.push(id),
            ResolvedGenericItem::GenericType(GenericTypeId::Extern(id)) => extern_types.push(id),
            ResolvedGenericItem::GenericFunction(GenericFunctionId::Extern(id)) => {
                extern_functions.push(id)
            }
            ResolvedGenericItem::Module(ModuleId::Submodule(id)) => submodules.push(id),
            ResolvedGenericItem::Module(ModuleId::CrateRoot(id)) => crates.push(id),
            _ => (),
        }
    }

    (
        constants,
        free_functions,
        structs,
        enums,
        module_type_aliases,
        impl_aliases,
        traits,
        impl_defs,
        extern_types,
        extern_functions,
        submodules,
        crates,
    )
}
