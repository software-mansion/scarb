// TODO(drknzz): Remove when not needed.
// #![allow(dead_code)]

use std::sync::Arc;

use anyhow::Result;
use cairo_lang_diagnostics::{DiagnosticAdded, Maybe};
use cairo_lang_semantic::items::visibility;
use cairo_lang_utils::ordered_hash_map::OrderedHashMap;
use cairo_lang_utils::Upcast;
use itertools::Itertools;
use serde::Serialize;

use cairo_lang_defs::db::DefsGroup;
use cairo_lang_defs::diagnostic_utils::StableLocation;
use cairo_lang_defs::ids::{
    ConstantId, EnumId, ExternFunctionId, ExternTypeId, FreeFunctionId, ImplAliasId,
    ImplConstantDefId, ImplDefId, ImplFunctionId, ImplItemId, ImplTypeDefId, LookupItemId,
    MemberId, ModuleId, ModuleItemId, ModuleTypeAliasId, NamedLanguageElementId, StructId,
    TopLevelLanguageElementId, TraitConstantId, TraitFunctionId, TraitId, TraitItemId, TraitTypeId,
    VariantId,
};
use cairo_lang_doc::db::DocGroup;
use cairo_lang_doc::documentable_item::DocumentableItemId;
use cairo_lang_filesystem::ids::CrateId;
use cairo_lang_semantic::db::SemanticGroup;
use cairo_lang_syntax::node::ast::{self};

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

impl Module {
    pub fn new(
        db: &ScarbDocDatabase,
        module_id: ModuleId,
        root_module_id: ModuleId,
        include_private_items: bool,
    ) -> Maybe<Self> {
        // FIXME(#1438): compiler doesn't support fetching root crate doc
        let item_data = match module_id {
            ModuleId::CrateRoot(crate_id) => ItemData {
                name: crate_id.name(db).to_string(),
                doc: None,
                signature: None,
                full_path: module_id.full_path(db),
            },
            ModuleId::Submodule(submodule_id) => ItemData::new_without_signature(
                db,
                submodule_id,
                LookupItemId::ModuleItem(ModuleItemId::Submodule(submodule_id)).into(),
            ),
        };

        let should_include_item = |id: &dyn TopLevelLanguageElementId| {
            if include_private_items {
                return Ok(true);
            }
            is_visible_in_module(db, root_module_id, id)
        };

        let module_constants = db.module_constants(module_id)?;
        let constants = filter_map_item_id_to_item(module_constants, should_include_item, |id| {
            Ok(Constant::new(db, id))
        })?;

        let module_free_functions = db.module_free_functions(module_id)?;
        let free_functions =
            filter_map_item_id_to_item(module_free_functions, should_include_item, |id| {
                Ok(FreeFunction::new(db, id))
            })?;

        let module_structs = db.module_structs(module_id)?;
        let structs = filter_map_item_id_to_item(module_structs, should_include_item, |id| {
            Struct::new(db, id, root_module_id, include_private_items)
        })?;

        let module_enums = db.module_enums(module_id)?;
        let enums =
            filter_map_item_id_to_item(module_enums, should_include_item, |id| Enum::new(db, id))?;

        let module_type_aliases = db.module_type_aliases(module_id)?;
        let type_aliases =
            filter_map_item_id_to_item(module_type_aliases, should_include_item, |id| {
                Ok(TypeAlias::new(db, id))
            })?;

        let module_impl_aliases = db.module_impl_aliases(module_id)?;
        let impl_aliases =
            filter_map_item_id_to_item(module_impl_aliases, should_include_item, |id| {
                Ok(ImplAlias::new(db, id))
            })?;

        let module_traits = db.module_traits(module_id)?;
        let traits = filter_map_item_id_to_item(module_traits, should_include_item, |id| {
            Trait::new(db, id)
        })?;

        let module_impls = db.module_impls(module_id)?;
        let impls =
            filter_map_item_id_to_item(module_impls, should_include_item, |id| Impl::new(db, id))?;

        let module_extern_types = db.module_extern_types(module_id)?;
        let extern_types =
            filter_map_item_id_to_item(module_extern_types, should_include_item, |id| {
                Ok(ExternType::new(db, id))
            })?;

        let module_extern_functions = db.module_extern_functions(module_id)?;
        let extern_functions =
            filter_map_item_id_to_item(module_extern_functions, should_include_item, |id| {
                Ok(ExternFunction::new(db, id))
            })?;

        let module_submodules = db.module_submodules(module_id)?;
        let submodules: Vec<Module> =
            filter_map_item_id_to_item(module_submodules, should_include_item, |id| {
                Module::new(
                    db,
                    ModuleId::Submodule(id),
                    root_module_id,
                    include_private_items,
                )
            })?;

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
/// J - Type representing an item ast type.
/// K - Type of generated item.
fn filter_map_item_id_to_item<T, F, G, K, J>(
    items: Arc<OrderedHashMap<T, J>>,
    should_include_item_function: F,
    generate_item_function: G,
) -> Result<Vec<K>, DiagnosticAdded>
where
    T: Copy + TopLevelLanguageElementId + ?Sized,
    F: Fn(&dyn TopLevelLanguageElementId) -> Result<bool, DiagnosticAdded>,
    G: Fn(T) -> Maybe<K>,
{
    items
        .iter()
        .filter_map(|(id, _)| match should_include_item_function(id) {
            Ok(result) => {
                if !result {
                    return None;
                }
                Some(Ok(generate_item_function(*id)))
            }
            Err(a) => Some(Err(a)),
        })
        .collect::<Maybe<Maybe<Vec<K>>>>()?
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

        let item_data = ItemData::new_without_signature(
            db,
            id,
            LookupItemId::ModuleItem(ModuleItemId::Struct(id)).into(),
        );

        let members = members
            .iter()
            .filter_map(|(_, semantic_member)| {
                match is_visible_in_module(db, root_module_id, &semantic_member.id) {
                    Ok(visible) => {
                        if include_private_items || visible {
                            Some(Ok(Member::new(
                                db,
                                semantic_member.id,
                                item_data.full_path.clone(),
                            )))
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
    pub fn new(db: &ScarbDocDatabase, id: MemberId, struct_full_path: String) -> Self {
        let node = id.stable_ptr(db);
        let stable_location = StableLocation::new(node.0);

        let name = id.name(db).into();
        // TODO(#1438): Replace with `id.full_path(db)` after it is fixed in the compiler.
        let full_path = format!("{}::{}", struct_full_path, name);

        let item_data = ItemData {
            name,
            doc: get_item_documentation(db, &stable_location),
            signature: None,
            full_path,
        };

        Self {
            id,
            node,
            item_data,
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
        let item_data = ItemData::new_without_signature(
            db,
            id,
            LookupItemId::ModuleItem(ModuleItemId::Enum(id)).into(),
        );

        let variants = variants
            .iter()
            .map(|(_name, variant_id)| Variant::new(db, *variant_id, item_data.full_path.clone()))
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
    pub fn new(db: &ScarbDocDatabase, id: VariantId, enum_full_path: String) -> Self {
        let node = id.stable_ptr(db);
        let stable_location = StableLocation::new(node.0);

        let name = id.name(db).into();
        // TODO(#1438): Replace with `id.full_path(db)` after it is fixed in the compiler.
        let full_path = format!("{}::{}", enum_full_path, name);

        let item_data = ItemData {
            name,
            doc: get_item_documentation(db, &stable_location),
            signature: None,
            full_path,
        };

        Self {
            id,
            node,
            item_data,
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
        let full_path_to_trait = match item_data.full_path.strip_suffix(item_data.name.as_str()) {
            Some(full_path) => full_path.to_string(),
            None => return Err(DiagnosticAdded),
        };

        let trait_constants = db.trait_constants(id)?;
        let trait_constants = trait_constants
            .iter()
            .map(|(_name, trait_constant_id)| {
                TraitConstant::new(db, *trait_constant_id, full_path_to_trait.clone())
            })
            .collect::<Vec<_>>();

        let trait_types = db.trait_types(id)?;
        let trait_types = trait_types
            .iter()
            .map(|(_name, trait_type_id)| {
                TraitType::new(db, *trait_type_id, full_path_to_trait.clone())
            })
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
    pub fn new(db: &ScarbDocDatabase, id: TraitConstantId, full_path_to_trait: String) -> Self {
        let node = id.stable_ptr(db);

        // FIXME(#1437): compiler returns empty string for a signature
        let mut item_data = ItemData::new(
            db,
            id,
            LookupItemId::TraitItem(TraitItemId::Constant(id)).into(),
        );
        // TODO(#1438): introduce proper fix in compiler
        item_data.full_path = full_path_to_trait + &item_data.full_path;

        Self {
            id,
            node,
            item_data,
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
    pub fn new(db: &ScarbDocDatabase, id: TraitTypeId, full_path_to_trait: String) -> Self {
        let node = id.stable_ptr(db);

        // FIXME(#1437): compiler returns empty string for a signature
        let mut item_data = ItemData::new(
            db,
            id,
            LookupItemId::TraitItem(TraitItemId::Type(id)).into(),
        );
        // TODO(#1438): introduce proper fix in compiler
        item_data.full_path = full_path_to_trait + &item_data.full_path;

        Self {
            id,
            node,
            item_data,
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
        let full_path_to_impl = match item_data.full_path.strip_suffix(item_data.name.as_str()) {
            Some(full_path) => full_path.to_string(),
            None => return Err(DiagnosticAdded),
        };

        let impl_types = db.impl_types(id)?;
        let impl_types = impl_types
            .iter()
            .map(|(id, _)| ImplType::new(db, *id, full_path_to_impl.clone()))
            .collect::<Vec<_>>();

        let impl_constants = db.impl_constants(id)?;
        let impl_constants = impl_constants
            .iter()
            .map(|(id, _)| ImplConstant::new(db, *id, full_path_to_impl.clone()))
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
    pub fn new(db: &ScarbDocDatabase, id: ImplTypeDefId, full_path_to_impl: String) -> Self {
        let node = id.stable_ptr(db);

        let mut item_data =
            ItemData::new(db, id, LookupItemId::ImplItem(ImplItemId::Type(id)).into());
        // TODO(#1438): introduce proper fix in compiler
        item_data.full_path = full_path_to_impl + &item_data.full_path;

        Self {
            id,
            node,
            item_data,
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
    pub fn new(db: &ScarbDocDatabase, id: ImplConstantDefId, full_path_to_impl: String) -> Self {
        let node = id.stable_ptr(db);

        let mut item_data = ItemData::new(
            db,
            id,
            LookupItemId::ImplItem(ImplItemId::Constant(id)).into(),
        );
        // TODO(#1438): introduce proper fix in compiler
        item_data.full_path = full_path_to_impl + &item_data.full_path;

        Self {
            id,
            node,
            item_data,
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

// TODO(#1428): This function is temporarily copied until further modifications in cairo compiler are done.
fn get_item_documentation(db: &dyn DefsGroup, stable_location: &StableLocation) -> Option<String> {
    let doc = stable_location.syntax_node(db).get_text(db.upcast());
    let doc = doc
        .lines()
        .take_while_ref(|line| {
            !line
                .trim_start()
                .chars()
                .next()
                .map_or(false, |c| c.is_alphabetic())
        })
        .filter_map(|line| {
            let dedent = line.trim_start();
            for prefix in ["///", "//!"] {
                if let Some(content) = dedent.strip_prefix(prefix) {
                    return Some(content.strip_prefix(' ').unwrap_or(content));
                }
            }
            None
        })
        .collect::<Vec<&str>>();
    (!doc.is_empty()).then(|| doc.join("\n"))
}
