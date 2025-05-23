use crate::db::ScarbDocDatabase;
use crate::types::groups::{
    Group, aggregate_constants_groups, aggregate_enums_groups, aggregate_extern_functions_groups,
    aggregate_extern_types_groups, aggregate_free_functions_groups, aggregate_impl_aliases_groups,
    aggregate_impls_groups, aggregate_modules_groups, aggregate_structs_groups,
    aggregate_traits_groups, aggregate_type_aliases_groups,
};
use crate::types::other_types::{
    Constant, Enum, ExternFunction, ExternType, FreeFunction, Impl, ImplAlias, ItemData, Struct,
    Trait, TypeAlias,
};
use cairo_lang_defs::db::DefsGroup;
use cairo_lang_defs::ids::{
    GenericTypeId, ImplDefId, LanguageElementId, LookupItemId, ModuleId, ModuleItemId,
    NamedLanguageElementId, TopLevelLanguageElementId,
};
use cairo_lang_diagnostics::{DiagnosticAdded, Maybe};
use cairo_lang_doc::documentable_item::DocumentableItemId;
use cairo_lang_semantic::db::SemanticGroup;
use cairo_lang_semantic::items::attribute::SemanticQueryAttrs;
use cairo_lang_semantic::items::functions::GenericFunctionId;
use cairo_lang_semantic::items::us::SemanticUseEx;
use cairo_lang_semantic::items::visibility::Visibility;
use cairo_lang_semantic::resolve::ResolvedGenericItem;
use cairo_lang_semantic::{ConcreteTypeId, GenericArgumentId, TypeLongId};
use cairo_lang_syntax::node::helpers::QueryAttrs;
use cairo_lang_syntax::node::{SyntaxNode, TypedSyntaxNode};
use cairo_lang_utils::LookupIntern;
use serde::Serialize;
use std::collections::HashMap;

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
    pub pub_uses: ModulePubUses,
    #[serde(skip_serializing)]
    pub groups: Vec<Group>,
}

#[derive(Clone, Default, Serialize)]
pub struct ModulePubUses {
    pub use_constants: Vec<Constant>,
    pub use_free_functions: Vec<FreeFunction>,
    pub use_structs: Vec<Struct>,
    pub use_enums: Vec<Enum>,
    pub use_module_type_aliases: Vec<TypeAlias>,
    pub use_impl_aliases: Vec<ImplAlias>,
    pub use_traits: Vec<Trait>,
    pub use_impl_defs: Vec<Impl>,
    pub use_extern_types: Vec<ExternType>,
    pub use_extern_functions: Vec<ExternFunction>,
    pub use_submodules: Vec<Module>,
}

impl ModulePubUses {
    pub fn new(
        db: &ScarbDocDatabase,
        module_id: ModuleId,
        include_private_items: bool,
    ) -> Maybe<Self> {
        let module_use_items: Vec<ResolvedGenericItem> = db
            .module_uses(module_id)?
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

        for item in module_use_items {
            match item {
                ResolvedGenericItem::GenericConstant(id) => {
                    use_constants.push(Constant::new(db, id))
                }
                ResolvedGenericItem::GenericFunction(GenericFunctionId::Free(id)) => {
                    use_free_functions.push(FreeFunction::new(db, id))
                }
                ResolvedGenericItem::GenericType(GenericTypeId::Struct(id)) => {
                    use_structs.push(Struct::new(db, id, include_private_items)?)
                }
                ResolvedGenericItem::GenericType(GenericTypeId::Enum(id)) => {
                    use_enums.push(Enum::new(db, id)?)
                }
                ResolvedGenericItem::GenericTypeAlias(id) => {
                    use_module_type_aliases.push(TypeAlias::new(db, id))
                }
                ResolvedGenericItem::GenericImplAlias(id) => {
                    use_impl_aliases.push(ImplAlias::new(db, id))
                }
                ResolvedGenericItem::Trait(id) => use_traits.push(Trait::new(db, id)?),
                ResolvedGenericItem::Impl(id) => use_impl_defs.push(Impl::new(db, id)?),
                ResolvedGenericItem::GenericType(GenericTypeId::Extern(id)) => {
                    use_extern_types.push(ExternType::new(db, id))
                }
                ResolvedGenericItem::GenericFunction(GenericFunctionId::Extern(id)) => {
                    use_extern_functions.push(ExternFunction::new(db, id))
                }
                ResolvedGenericItem::Module(ModuleId::Submodule(id)) => use_submodules.push(
                    Module::new(db, ModuleId::Submodule(id), include_private_items)?,
                ),
                ResolvedGenericItem::Module(ModuleId::CrateRoot(id)) => use_submodules.push(
                    Module::new(db, ModuleId::CrateRoot(id), include_private_items)?,
                ),
                _ => (),
            }
        }

        Ok(Self {
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
        })
    }
}

macro_rules! define_insert_function {
    (
        $(
            $fn_name:ident,
            $field_name:ident,
            $item_type:ty
        );*
    ) => {
        $(
            pub fn $fn_name(&mut self, item: $item_type) {
                if self
                    .$field_name
                    .iter()
                    .any(|existing_item| existing_item.id == item.id)
                {
                    return;
                }
                self.$field_name.push(item);
            }
        )*
    };
}

impl Module {
    define_insert_function!(
        insert_constant, constants, Constant;
        insert_free_function, free_functions, FreeFunction;
        insert_struct, structs, Struct;
        insert_enum, enums, Enum;
        insert_type_alias, type_aliases, TypeAlias;
        insert_impl_alias, impl_aliases, ImplAlias;
        insert_trait, traits, Trait;
        insert_impl, impls, Impl;
        insert_extern_type, extern_types, ExternType;
        insert_extern_function, extern_functions, ExternFunction
    );

    pub fn new(
        db: &ScarbDocDatabase,
        module_id: ModuleId,
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
            let syntax_node = id.stable_location(db).syntax_node(db);

            Ok((include_private_items || is_public(db, id)?)
                && !is_doc_hidden_attr(db, &syntax_node))
        };

        let module_pubuses = ModulePubUses::new(db, module_id, include_private_items)?;

        let module_constants = db.module_constants(module_id)?;
        let mut constants =
            filter_map_item_id_to_item(module_constants.keys(), should_include_item, |id| {
                Ok(Constant::new(db, *id))
            })?;

        let module_free_functions = db.module_free_functions(module_id)?;

        let mut free_functions =
            filter_map_item_id_to_item(module_free_functions.keys(), should_include_item, |id| {
                Ok(FreeFunction::new(db, *id))
            })?;

        let module_structs = db.module_structs(module_id)?;
        let mut structs =
            filter_map_item_id_to_item(module_structs.keys(), should_include_item, |id| {
                Struct::new(db, *id, include_private_items)
            })?;

        let module_enums = db.module_enums(module_id)?;
        let mut enums =
            filter_map_item_id_to_item(module_enums.keys(), should_include_item, |id| {
                Enum::new(db, *id)
            })?;

        let module_type_aliases = db.module_type_aliases(module_id)?;
        let mut type_aliases =
            filter_map_item_id_to_item(module_type_aliases.keys(), should_include_item, |id| {
                Ok(TypeAlias::new(db, *id))
            })?;

        let module_impl_aliases = db.module_impl_aliases(module_id)?;
        let mut impl_aliases =
            filter_map_item_id_to_item(module_impl_aliases.keys(), should_include_item, |id| {
                Ok(ImplAlias::new(db, *id))
            })?;

        let module_traits = db.module_traits(module_id)?;
        let mut traits =
            filter_map_item_id_to_item(module_traits.keys(), should_include_item, |id| {
                Trait::new(db, *id)
            })?;

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
                    let args = concrete_trait_id.generic_args(db);
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
        let mut impls = filter_map_item_id_to_item(
            module_impls.keys().filter(hide_impls_for_hidden_traits),
            should_include_item,
            |id| Impl::new(db, *id),
        )?;

        let module_extern_types = db.module_extern_types(module_id)?;
        let mut extern_types =
            filter_map_item_id_to_item(module_extern_types.keys(), should_include_item, |id| {
                Ok(ExternType::new(db, *id))
            })?;

        let module_extern_functions = db.module_extern_functions(module_id)?;
        let mut extern_functions = filter_map_item_id_to_item(
            module_extern_functions.keys(),
            should_include_item,
            |id| Ok(ExternFunction::new(db, *id)),
        )?;
        let module_submodules = db.module_submodules(module_id)?;
        let mut submodules: Vec<Module> =
            filter_map_item_id_to_item(module_submodules.keys(), should_include_item, |id| {
                Module::new(db, ModuleId::Submodule(*id), include_private_items)
            })?;

        let mut group_map: HashMap<String, Group> = HashMap::new();
        constants = aggregate_constants_groups(&constants, &mut group_map);
        free_functions = aggregate_free_functions_groups(&free_functions, &mut group_map);
        structs = aggregate_structs_groups(&structs, &mut group_map);
        enums = aggregate_enums_groups(&enums, &mut group_map);
        type_aliases = aggregate_type_aliases_groups(&type_aliases, &mut group_map);
        impl_aliases = aggregate_impl_aliases_groups(&impl_aliases, &mut group_map);
        traits = aggregate_traits_groups(&traits, &mut group_map);
        impls = aggregate_impls_groups(&impls, &mut group_map);
        extern_types = aggregate_extern_types_groups(&extern_types, &mut group_map);
        extern_functions = aggregate_extern_functions_groups(&extern_functions, &mut group_map);
        submodules = aggregate_modules_groups(&submodules, &mut group_map);
        let mut groups: Vec<Group> = group_map.into_values().collect();
        groups.sort_by(|a, b| a.name.cmp(&b.name));

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
            pub_uses: module_pubuses,
            groups,
        })
    }

    pub(crate) fn new_virtual(db: &ScarbDocDatabase, module_id: ModuleId) -> Self {
        let item_data = match module_id {
            ModuleId::CrateRoot(crate_id) => ItemData::new_crate(db, crate_id),
            ModuleId::Submodule(submodule_id) => ItemData::new_without_signature(
                db,
                submodule_id,
                LookupItemId::ModuleItem(ModuleItemId::Submodule(submodule_id)).into(),
            ),
        };
        Self {
            module_id,
            item_data,
            submodules: Default::default(),
            constants: Default::default(),
            free_functions: Default::default(),
            structs: Default::default(),
            enums: Default::default(),
            type_aliases: Default::default(),
            impl_aliases: Default::default(),
            traits: Default::default(),
            impls: Default::default(),
            extern_types: Default::default(),
            extern_functions: Default::default(),
            pub_uses: Default::default(),
            groups: vec![],
        }
    }

    /// Recursively traverses all the module and gets all the item [`DocumentableItemId`]s.
    pub(crate) fn get_all_item_ids(&self) -> HashMap<DocumentableItemId, &ItemData> {
        let mut ids: HashMap<DocumentableItemId, &ItemData> = HashMap::default();

        ids.insert(self.item_data.id, &self.item_data);
        self.constants.iter().for_each(|item| {
            ids.insert(item.item_data.id, &item.item_data);
        });
        self.free_functions.iter().for_each(|item| {
            ids.insert(item.item_data.id, &item.item_data);
        });
        self.type_aliases.iter().for_each(|item| {
            ids.insert(item.item_data.id, &item.item_data);
        });
        self.impl_aliases.iter().for_each(|item| {
            ids.insert(item.item_data.id, &item.item_data);
        });
        self.free_functions.iter().for_each(|item| {
            ids.insert(item.item_data.id, &item.item_data);
        });
        self.extern_types.iter().for_each(|item| {
            ids.insert(item.item_data.id, &item.item_data);
        });
        self.extern_functions.iter().for_each(|item| {
            ids.insert(item.item_data.id, &item.item_data);
        });

        self.structs.iter().for_each(|struct_| {
            ids.insert(struct_.item_data.id, &struct_.item_data);
            struct_.get_all_item_ids();
        });

        self.enums.iter().for_each(|enum_| {
            ids.insert(enum_.item_data.id, &enum_.item_data);
            ids.extend(enum_.get_all_item_ids());
        });

        self.traits.iter().for_each(|trait_| {
            ids.insert(trait_.item_data.id, &trait_.item_data);
            ids.extend(trait_.get_all_item_ids());
        });

        self.impls.iter().for_each(|impl_| {
            ids.insert(impl_.item_data.id, &impl_.item_data);
            ids.extend(impl_.get_all_item_ids());
        });

        self.submodules.iter().for_each(|sub_module| {
            ids.extend(sub_module.get_all_item_ids());
        });

        ids
    }
}

/// Merges subitems of virtual_module into documented_module so it contains all unique data from both modules.
/// Note that documented_module might have been created by [`Module::new_virtual`].   
pub(crate) fn merge_modules(documented_module: &mut Module, virtual_module: Module) -> &mut Module {
    for constant in virtual_module.constants {
        documented_module.insert_constant(constant);
    }
    for free_function in virtual_module.free_functions {
        documented_module.insert_free_function(free_function);
    }
    for struct_ in virtual_module.structs {
        documented_module.insert_struct(struct_);
    }
    for enum_ in virtual_module.enums {
        documented_module.insert_enum(enum_);
    }
    for type_alias in virtual_module.type_aliases {
        documented_module.insert_type_alias(type_alias);
    }
    for impl_alias in virtual_module.impl_aliases {
        documented_module.insert_impl_alias(impl_alias);
    }
    for trait_ in virtual_module.traits {
        documented_module.insert_trait(trait_);
    }
    for impl_ in virtual_module.impls {
        documented_module.insert_impl(impl_);
    }
    for extern_type in virtual_module.extern_types {
        documented_module.insert_extern_type(extern_type);
    }
    for extern_function in virtual_module.extern_functions {
        documented_module.insert_extern_function(extern_function);
    }
    for submodule2 in virtual_module.submodules {
        if let Some(submodule_index) = documented_module
            .submodules
            .iter()
            .position(|submodule1| submodule1.module_id == submodule2.module_id)
        {
            merge_modules(
                &mut documented_module.submodules[submodule_index],
                submodule2,
            );
        } else {
            documented_module.submodules.push(submodule2);
        }
    }
    documented_module
}

pub(crate) fn get_ancestors_vector(
    ancestors: &mut Vec<ModuleId>,
    module_id: ModuleId,
    db: &ScarbDocDatabase,
) -> Vec<ModuleId> {
    if let ModuleId::Submodule(submodule_id) = module_id {
        ancestors.insert(0, module_id);
        let parent = submodule_id.parent_module(db);
        get_ancestors_vector(ancestors, parent, db);
    }
    ancestors.clone()
}

pub(crate) fn collect_pubuses(all_pub_uses: &mut ModulePubUses, module: Module) -> ModulePubUses {
    all_pub_uses
        .use_constants
        .extend(module.pub_uses.use_constants);
    all_pub_uses
        .use_free_functions
        .extend(module.pub_uses.use_free_functions);
    all_pub_uses.use_structs.extend(module.pub_uses.use_structs);
    all_pub_uses.use_enums.extend(module.pub_uses.use_enums);
    all_pub_uses
        .use_module_type_aliases
        .extend(module.pub_uses.use_module_type_aliases);
    all_pub_uses
        .use_impl_aliases
        .extend(module.pub_uses.use_impl_aliases);
    all_pub_uses.use_traits.extend(module.pub_uses.use_traits);
    all_pub_uses
        .use_impl_defs
        .extend(module.pub_uses.use_impl_defs);
    all_pub_uses
        .use_extern_types
        .extend(module.pub_uses.use_extern_types);
    all_pub_uses
        .use_extern_functions
        .extend(module.pub_uses.use_extern_functions);
    all_pub_uses
        .use_submodules
        .extend(module.pub_uses.use_submodules);

    for submodule in module.submodules {
        collect_pubuses(all_pub_uses, submodule);
    }
    all_pub_uses.to_owned()
}

pub(crate) fn is_doc_hidden_attr(db: &ScarbDocDatabase, syntax_node: &SyntaxNode) -> bool {
    syntax_node.has_attr_with_arg(db, "doc", "hidden")
}

fn is_public(db: &ScarbDocDatabase, element_id: &dyn TopLevelLanguageElementId) -> Maybe<bool> {
    let containing_module_id = element_id.parent_module(db);
    match db.module_item_info_by_name(containing_module_id, element_id.name(db))? {
        Some(module_item_info) => Ok(matches!(module_item_info.visibility, Visibility::Public)),
        None => Ok(false),
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
) -> anyhow::Result<Vec<K>, DiagnosticAdded>
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

fn is_doc_hidden_attr_semantic(
    db: &dyn SemanticGroup,
    node: &dyn SemanticQueryAttrs,
) -> Maybe<bool> {
    node.has_attr_with_arg(db, "doc", "hidden")
}
