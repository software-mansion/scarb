use anyhow::Result;
use cairo_lang_diagnostics::{DiagnosticAdded, Maybe};
use cairo_lang_semantic::items::functions::GenericFunctionId;
use cairo_lang_semantic::items::us::SemanticUseEx;
use cairo_lang_semantic::items::visibility::{self, Visibility};
use cairo_lang_semantic::resolve::ResolvedGenericItem;
use cairo_lang_syntax::node::helpers::QueryAttrs;
use cairo_lang_utils::{LookupIntern, Upcast};
use itertools::chain;
use serde::Serialize;

use cairo_lang_defs::db::DefsGroup;
use cairo_lang_defs::ids::{
    ConstantId, EnumId, ExternFunctionId, ExternTypeId, FreeFunctionId, GenericTypeId, ImplAliasId,
    ImplConstantDefId, ImplDefId, ImplFunctionId, ImplItemId, ImplTypeDefId, LanguageElementId,
    LookupItemId, MemberId, ModuleId, ModuleItemId, ModuleTypeAliasId, NamedLanguageElementId,
    StructId, SubmoduleId, TopLevelLanguageElementId, TraitConstantId, TraitFunctionId, TraitId,
    TraitItemId, TraitTypeId, VariantId,
};
use cairo_lang_doc::db::DocGroup;
use cairo_lang_doc::documentable_item::DocumentableItemId;
use cairo_lang_filesystem::ids::CrateId;
use cairo_lang_semantic::db::SemanticGroup;
use cairo_lang_semantic::items::attribute::SemanticQueryAttrs;
use cairo_lang_semantic::{ConcreteTypeId, GenericArgumentId, TypeLongId};
use cairo_lang_syntax::node::{
    ast::{self},
    SyntaxNode, TypedSyntaxNode,
};

use crate::db::ScarbDocDatabase;

#[derive(Serialize, Clone)]
pub struct Crate {
    pub root_module: Module,
}

impl Crate {
    pub fn new(
        db: &ScarbDocDatabase,
        crate_id: CrateId,
        include_private_items: bool,
    ) -> Maybe<Self> {
        let root_module_id = ModuleId::CrateRoot(crate_id);
        Ok(Self {
            root_module: Module::new(db, root_module_id, root_module_id, include_private_items)?,
        })
    }
}

fn is_visible_in_module(
    db: &ScarbDocDatabase,
    root_module_id: ModuleId,
    element_id: &dyn TopLevelLanguageElementId,
) -> Maybe<bool> {
    let cotaining_module_id = element_id.parent_module(db);
    match db.module_item_info_by_name(cotaining_module_id, element_id.name(db.upcast()))? {
        Some(module_item_info) => Ok(visibility::peek_visible_in(
            db,
            module_item_info.visibility,
            cotaining_module_id,
            root_module_id,
        )),
        None => Ok(false),
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

struct ModulePubUses {
    pub use_constants: Vec<ConstantId>,
    pub use_free_functions: Vec<FreeFunctionId>,
    pub use_structs: Vec<StructId>,
    pub use_enums: Vec<EnumId>,
    pub use_module_type_aliases: Vec<ModuleTypeAliasId>,
    pub use_impl_aliases: Vec<ImplAliasId>,
    pub use_traits: Vec<TraitId>,
    pub use_impl_defs: Vec<ImplDefId>,
    pub use_extern_types: Vec<ExternTypeId>,
    pub use_extern_functions: Vec<ExternFunctionId>,
    pub use_submodules: Vec<SubmoduleId>,
    pub use_crates: Vec<CrateId>,
}

impl ModulePubUses {
    pub fn new(db: &ScarbDocDatabase, module_id: ModuleId) -> Self {
        let module_use_items: Vec<ResolvedGenericItem> = db
            .module_uses(module_id)
            .unwrap()
            .iter()
            .filter_map(|(use_id, _)| {
                let visibility = db
                    .module_item_info_by_name(module_id, use_id.name(db))
                    .unwrap()
                    .unwrap()
                    .visibility;
                if visibility == Visibility::Public {
                    Some(db.use_resolved_item(*use_id).unwrap())
                } else {
                    None
                }
            })
            .collect();

        let mut use_constants = Vec::new();
        let mut use_free_functions = Vec::new();
        let mut use_structs = Vec::new();
        let mut use_enums = Vec::new();
        let mut use_module_type_aliases = Vec::new();
        let mut use_impl_aliases = Vec::new();
        let mut use_traits = Vec::new();
        let mut use_impl_defs = Vec::new();
        let mut use_extern_types = Vec::new();
        let mut use_extern_functions = Vec::new();
        let mut use_submodules = Vec::new();
        let mut use_crates = Vec::new();

        for item in module_use_items {
            match item {
                ResolvedGenericItem::GenericConstant(id) => use_constants.push(id),
                ResolvedGenericItem::GenericFunction(GenericFunctionId::Free(id)) => {
                    use_free_functions.push(id)
                }
                ResolvedGenericItem::GenericType(GenericTypeId::Struct(id)) => use_structs.push(id),
                ResolvedGenericItem::GenericType(GenericTypeId::Enum(id)) => use_enums.push(id),
                ResolvedGenericItem::GenericTypeAlias(id) => use_module_type_aliases.push(id),
                ResolvedGenericItem::GenericImplAlias(id) => use_impl_aliases.push(id),
                ResolvedGenericItem::Trait(id) => use_traits.push(id),
                ResolvedGenericItem::Impl(id) => use_impl_defs.push(id),
                ResolvedGenericItem::GenericType(GenericTypeId::Extern(id)) => {
                    use_extern_types.push(id)
                }
                ResolvedGenericItem::GenericFunction(GenericFunctionId::Extern(id)) => {
                    use_extern_functions.push(id)
                }
                ResolvedGenericItem::Module(ModuleId::Submodule(id)) => use_submodules.push(id),
                ResolvedGenericItem::Module(ModuleId::CrateRoot(id)) => use_crates.push(id),
                _ => (),
            }
        }

        Self {
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
        }
    }
}

impl Module {
    pub fn new(
        db: &ScarbDocDatabase,
        module_id: ModuleId,
        root_module_id: ModuleId,
        include_private_items: bool,
    ) -> Maybe<Self> {
        let item_data = match module_id {
            ModuleId::CrateRoot(crate_id) => ItemData::new_crate(db, crate_id),
            ModuleId::Submodule(submodule_id) => ItemData::new_without_signature(
                db,
                submodule_id,
                LookupItemId::ModuleItem(ModuleItemId::Submodule(submodule_id)).into(),
            ),
        };

        let should_include_item = |id: &dyn TopLevelLanguageElementId| {
            let syntax_node = id.stable_location(db.upcast()).syntax_node(db.upcast());

            Ok(
                (include_private_items || is_visible_in_module(db, root_module_id, id)?)
                    && !is_doc_hidden_attr(db, &syntax_node),
            )
        };

        let module_pubuses = ModulePubUses::new(db, module_id);

        let module_constants = db.module_constants(module_id)?;
        let constants = filter_map_item_id_to_item(
            chain!(module_constants.keys(), module_pubuses.use_constants.iter()),
            should_include_item,
            |id| Ok(Constant::new(db, *id)),
        )?;

        let module_free_functions = db.module_free_functions(module_id)?;
        let free_functions = filter_map_item_id_to_item(
            chain!(
                module_free_functions.keys(),
                module_pubuses.use_free_functions.iter()
            ),
            should_include_item,
            |id| Ok(FreeFunction::new(db, *id)),
        )?;

        let module_structs = db.module_structs(module_id)?;
        let structs = filter_map_item_id_to_item(
            chain!(module_structs.keys(), module_pubuses.use_structs.iter()),
            should_include_item,
            |id| Struct::new(db, *id, root_module_id, include_private_items),
        )?;

        let module_enums = db.module_enums(module_id)?;
        let enums = filter_map_item_id_to_item(
            chain!(module_enums.keys(), module_pubuses.use_enums.iter()),
            should_include_item,
            |id| Enum::new(db, *id),
        )?;

        let module_type_aliases = db.module_type_aliases(module_id)?;
        let type_aliases = filter_map_item_id_to_item(
            chain!(
                module_type_aliases.keys(),
                module_pubuses.use_module_type_aliases.iter()
            ),
            should_include_item,
            |id| Ok(TypeAlias::new(db, *id)),
        )?;

        let module_impl_aliases = db.module_impl_aliases(module_id)?;
        let impl_aliases = filter_map_item_id_to_item(
            chain!(
                module_impl_aliases.keys(),
                module_pubuses.use_impl_aliases.iter()
            ),
            should_include_item,
            |id| Ok(ImplAlias::new(db, *id)),
        )?;

        let module_traits = db.module_traits(module_id)?;
        let traits = filter_map_item_id_to_item(
            chain!(module_traits.keys(), module_pubuses.use_traits.iter()),
            should_include_item,
            |id| Trait::new(db, *id),
        )?;

        let module_impls = db.module_impls(module_id)?;
        let hide_impls_for_hidden_traits = |impl_def_id: &&ImplDefId| {
            // Hide impls for hidden traits and hidden trait generic args.
            // Example: `HiddenTrait<*>` or `NotHiddenTrait<HiddenStruct>` (e.g. Drop<HiddenStruct>).
            // We still keep impls, if any trait generic argument is not hidden.
            let Ok(trait_id) = db.impl_def_trait(**impl_def_id) else {
                return true;
            };
            let Ok(item_trait) = db.module_trait_by_id(trait_id) else {
                return true;
            };
            let all_generic_args_are_hidden = db
                .impl_def_concrete_trait(**impl_def_id)
                .ok()
                .map(|concrete_trait_id| {
                    let args = concrete_trait_id.generic_args(db.upcast());
                    if args.is_empty() {
                        return false;
                    }
                    args.into_iter()
                        .filter_map(|arg_id| {
                            let GenericArgumentId::Type(type_id) = arg_id else {
                                return None;
                            };
                            let TypeLongId::Concrete(concrete_type_id) = type_id.lookup_intern(db)
                            else {
                                return None;
                            };
                            let concrete_id: &dyn SemanticQueryAttrs = match &concrete_type_id {
                                ConcreteTypeId::Struct(struct_id) => struct_id,
                                ConcreteTypeId::Enum(enum_id) => enum_id,
                                ConcreteTypeId::Extern(extern_type_id) => extern_type_id,
                            };
                            is_doc_hidden_attr_semantic(db, concrete_id).ok()
                        })
                        .all(|x| x)
                })
                .unwrap_or(false);

            let trait_is_hidden = item_trait
                .map(|item_trait| is_doc_hidden_attr(db, &item_trait.as_syntax_node()))
                .unwrap_or(false);

            !(all_generic_args_are_hidden || trait_is_hidden)
        };
        let impls = filter_map_item_id_to_item(
            chain!(module_impls.keys(), module_pubuses.use_impl_defs.iter())
                .filter(hide_impls_for_hidden_traits),
            should_include_item,
            |id| Impl::new(db, *id),
        )?;

        let module_extern_types = db.module_extern_types(module_id)?;
        let extern_types = filter_map_item_id_to_item(
            chain!(
                module_extern_types.keys(),
                module_pubuses.use_extern_types.iter()
            ),
            should_include_item,
            |id| Ok(ExternType::new(db, *id)),
        )?;

        let module_extern_functions = db.module_extern_functions(module_id)?;
        let extern_functions = filter_map_item_id_to_item(
            chain!(
                module_extern_functions.keys(),
                module_pubuses.use_extern_functions.iter()
            ),
            should_include_item,
            |id| Ok(ExternFunction::new(db, *id)),
        )?;

        let module_submodules = db.module_submodules(module_id)?;
        let mut submodules: Vec<Module> = filter_map_item_id_to_item(
            chain!(
                module_submodules.keys(),
                module_pubuses.use_submodules.iter()
            ),
            should_include_item,
            |id| {
                Module::new(
                    db,
                    ModuleId::Submodule(*id),
                    root_module_id,
                    include_private_items,
                )
            },
        )?;

        let reexported_crates_as_modules: Vec<Module> = module_pubuses
            .use_crates
            .iter()
            .map(|id| {
                Module::new(
                    db,
                    ModuleId::CrateRoot(*id),
                    root_module_id,
                    include_private_items,
                )
            })
            .collect::<Maybe<_>>()?;

        submodules.extend(reexported_crates_as_modules);

        Ok(Self {
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
        })
    }
}

/// Takes the HashMap of items (returned from db query), filter them based on the `should_include_item_function` returned value,
/// and then generates an item based on its ID with function `generate_item_function`.
/// Generic types:
/// T - Type representing ID of an item. Accepts any kind of `TopLevelLanguageElementId`.
/// F - function that checks whether the id should be included in the result Vector.
/// G - A closure (as a function), which generates an item based on the item's ID.
/// K - Type of generated item.
fn filter_map_item_id_to_item<'a, T, F, G, K>(
    items: impl Iterator<Item = &'a T>,
    should_include_item_function: F,
    generate_item_function: G,
) -> Result<Vec<K>, DiagnosticAdded>
where
    T: 'a + Copy + TopLevelLanguageElementId,
    F: Fn(&'a dyn TopLevelLanguageElementId) -> Result<bool, DiagnosticAdded>,
    G: Fn(&'a T) -> Maybe<K>,
{
    items
        .filter_map(|id| match should_include_item_function(id) {
            Ok(result) => {
                if !result {
                    return None;
                }
                Some(Ok(generate_item_function(id)))
            }
            Err(a) => Some(Err(a)),
        })
        .collect::<Maybe<Maybe<Vec<K>>>>()?
}

fn is_doc_hidden_attr(db: &ScarbDocDatabase, syntax_node: &SyntaxNode) -> bool {
    syntax_node.has_attr_with_arg(db, "doc", "hidden")
}

fn is_doc_hidden_attr_semantic(
    db: &dyn SemanticGroup,
    node: &dyn SemanticQueryAttrs,
) -> Maybe<bool> {
    node.has_attr_with_arg(db, "doc", "hidden")
}

#[derive(Debug, Serialize, Clone)]
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
    ) -> Maybe<Self> {
        let members = db.struct_members(id)?;

        let item_data = ItemData::new(
            db,
            id,
            LookupItemId::ModuleItem(ModuleItemId::Struct(id)).into(),
        );
        let members = members
            .iter()
            .filter_map(|(_, semantic_member)| {
                match is_visible_in_module(db, root_module_id, &semantic_member.id) {
                    Ok(visible) => {
                        let syntax_node = &semantic_member
                            .id
                            .stable_location(db.upcast())
                            .syntax_node(db.upcast());
                        if (include_private_items || visible)
                            && !is_doc_hidden_attr(db, syntax_node)
                        {
                            Some(Ok(Member::new(db, semantic_member.id)))
                        } else {
                            None
                        }
                    }
                    Err(e) => Some(Err(e)),
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
