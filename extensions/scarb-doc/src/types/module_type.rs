use crate::db::ScarbDocDatabase;
use crate::types::groups::{
    Group, aggregate_constants_groups, aggregate_enums_groups, aggregate_extern_functions_groups,
    aggregate_extern_types_groups, aggregate_free_functions_groups, aggregate_impl_aliases_groups,
    aggregate_impls_groups, aggregate_modules_groups, aggregate_pub_uses_groups,
    aggregate_structs_groups, aggregate_traits_groups, aggregate_type_aliases_groups,
};
use crate::types::other_types::{
    Constant, Enum, ExternFunction, ExternType, FreeFunction, Impl, ImplAlias, ItemData,
    MacroDeclaration, Struct, Trait, TypeAlias,
};
use cairo_lang_defs::db::DefsGroup;
use cairo_lang_defs::ids::{
    GenericTypeId, ImplDefId, LanguageElementId, LookupItemId, ModuleId, ModuleItemId,
    NamedLanguageElementLongId, TopLevelLanguageElementId,
};
use cairo_lang_diagnostics::{DiagnosticAdded, Maybe};
use cairo_lang_doc::documentable_item::DocumentableItemId;
use cairo_lang_semantic::items::attribute::SemanticQueryAttrs;
use cairo_lang_semantic::items::functions::GenericFunctionId;
use cairo_lang_semantic::items::imp::ImplSemantic;
use cairo_lang_semantic::items::macro_call::module_macro_modules;
use cairo_lang_semantic::items::module::ModuleSemantic;
use cairo_lang_semantic::items::us::SemanticUseEx;
use cairo_lang_semantic::items::visibility::Visibility;
use cairo_lang_semantic::resolve::ResolvedGenericItem;
use cairo_lang_semantic::{ConcreteTypeId, GenericArgumentId, TypeLongId};
use cairo_lang_syntax::node::helpers::QueryAttrs;
use cairo_lang_syntax::node::{SyntaxNode, TypedSyntaxNode};
use serde::Serialize;
use std::collections::HashMap;

#[derive(Serialize, Clone)]
pub struct Module<'db> {
    #[serde(skip)]
    pub module_id: ModuleId<'db>,
    pub item_data: ItemData<'db>,

    pub submodules: Vec<Module<'db>>,
    pub constants: Vec<Constant<'db>>,
    pub free_functions: Vec<FreeFunction<'db>>,
    pub structs: Vec<Struct<'db>>,
    pub enums: Vec<Enum<'db>>,
    pub type_aliases: Vec<TypeAlias<'db>>,
    pub impl_aliases: Vec<ImplAlias<'db>>,
    pub traits: Vec<Trait<'db>>,
    pub impls: Vec<Impl<'db>>,
    pub extern_types: Vec<ExternType<'db>>,
    pub extern_functions: Vec<ExternFunction<'db>>,
    pub pub_uses: ModulePubUses<'db>,
    pub macro_declarations: Vec<MacroDeclaration<'db>>,
    #[serde(skip_serializing)]
    pub groups: Vec<Group<'db>>,
}

#[derive(Clone, Default, Serialize)]
pub struct ModulePubUses<'db> {
    pub use_constants: Vec<Constant<'db>>,
    pub use_free_functions: Vec<FreeFunction<'db>>,
    pub use_structs: Vec<Struct<'db>>,
    pub use_enums: Vec<Enum<'db>>,
    pub use_module_type_aliases: Vec<TypeAlias<'db>>,
    pub use_impl_aliases: Vec<ImplAlias<'db>>,
    pub use_traits: Vec<Trait<'db>>,
    pub use_impl_defs: Vec<Impl<'db>>,
    pub use_extern_types: Vec<ExternType<'db>>,
    pub use_extern_functions: Vec<ExternFunction<'db>>,
    pub use_submodules: Vec<Module<'db>>,
    pub use_macro_declarations: Vec<MacroDeclaration<'db>>,
}

impl<'db> ModulePubUses<'db> {
    pub fn new(
        db: &'db ScarbDocDatabase,
        module_id: ModuleId<'db>,
        include_private_items: bool,
    ) -> Maybe<Self> {
        let module_use_items: Vec<ResolvedGenericItem> = module_id
            .module_data(db)?
            .uses(db)
            .iter()
            .filter_map(|(use_id, _)| {
                db.module_item_info_by_name(module_id, use_id.long(db).name(db))
                    .ok()
                    .flatten()
                    .filter(|info| matches!(info.visibility, Visibility::Public))
                    .and_then(|_| db.use_resolved_item(*use_id).ok())
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
        let mut use_macro_declarations = Vec::new();

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
                ResolvedGenericItem::Macro(id) => {
                    use_macro_declarations.push(MacroDeclaration::new(db, id))
                }
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
            use_macro_declarations,
        })
    }

    pub fn is_empty(&self) -> bool {
        self.use_constants.is_empty()
            && self.use_free_functions.is_empty()
            && self.use_structs.is_empty()
            && self.use_enums.is_empty()
            && self.use_module_type_aliases.is_empty()
            && self.use_impl_aliases.is_empty()
            && self.use_traits.is_empty()
            && self.use_impl_defs.is_empty()
            && self.use_extern_types.is_empty()
            && self.use_extern_functions.is_empty()
            && self.use_submodules.is_empty()
            && self.use_macro_declarations.is_empty()
    }

    fn add(&mut self, other: Self) {
        self.use_constants.extend(other.use_constants);
        self.use_free_functions.extend(other.use_free_functions);
        self.use_structs.extend(other.use_structs);
        self.use_enums.extend(other.use_enums);
        self.use_module_type_aliases
            .extend(other.use_module_type_aliases);
        self.use_impl_aliases.extend(other.use_impl_aliases);
        self.use_traits.extend(other.use_traits);
        self.use_impl_defs.extend(other.use_impl_defs);
        self.use_extern_types.extend(other.use_extern_types);
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
                } else if let Some(item_group_name)  = item.item_data.group.as_ref() {
                    // avoid duplicating items in module.groups and module.pub_uses
                    for group in self.groups.iter_mut() {
                        if &group.name == item_group_name {
                            for existing_item in group.$field_name.iter() {
                                if existing_item.id == item.id {
                                    // PubUses do not guarantee uniquness, and Group must do so.
                                    return;
                                }
                            }
                            group.$field_name.push(item);
                            return;
                        }
                    }

                    let mut group = Group {
                        name: item_group_name.clone(),
                        submodules: vec![],
                        constants: vec![],
                        free_functions: vec![],
                        structs: vec![],
                        enums: vec![],
                        type_aliases: vec![],
                        impl_aliases: vec![],
                        traits: vec![],
                        impls: vec![],
                        extern_types: vec![],
                        extern_functions: vec![],
                        macro_declarations: vec![],
                    };
                    group.$field_name.push(item);
                    self.groups.push(group);
                    return;

                } else {
                self.$field_name.push(item);
                }
            }
        )*
    };
}

impl<'db> Module<'db> {
    define_insert_function!(
        insert_constant, constants, Constant<'db>;
        insert_free_function, free_functions, FreeFunction<'db>;
        insert_struct, structs, Struct<'db>;
        insert_enum, enums, Enum<'db>;
        insert_type_alias, type_aliases, TypeAlias<'db>;
        insert_impl_alias, impl_aliases, ImplAlias<'db>;
        insert_trait, traits, Trait<'db>;
        insert_impl, impls, Impl<'db>;
        insert_extern_type, extern_types, ExternType<'db>;
        insert_extern_function, extern_functions, ExternFunction<'db>;
        insert_macro_declaration, macro_declarations, MacroDeclaration<'db>
    );

    pub fn new(
        db: &'db ScarbDocDatabase,
        module_id: ModuleId<'db>,
        include_private_items: bool,
    ) -> Maybe<Self> {
        let item_data = match module_id {
            ModuleId::CrateRoot(crate_id) => ItemData::new_crate(db, crate_id),
            ModuleId::Submodule(submodule_id) => ItemData::new_without_signature(
                db,
                submodule_id,
                LookupItemId::ModuleItem(ModuleItemId::Submodule(submodule_id)).into(),
            ),
            ModuleId::MacroCall { .. } => {
                panic!("error: Module::new should not be called for MacroCall")
            }
        };

        let should_include_item = |id: &dyn TopLevelLanguageElementId<'db>| {
            let syntax_node = id.stable_location(db).syntax_node(db);

            Ok((include_private_items || is_public(db, id)?)
                && !is_doc_hidden_attr(db, &syntax_node))
        };

        let module_data = module_id.module_data(db)?;

        let mut constants = filter_map_item_id_to_item(
            module_data.constants(db).keys(),
            should_include_item,
            |id| Ok(Constant::new(db, *id)),
        )?;
        let mut free_functions = filter_map_item_id_to_item(
            module_data.free_functions(db).keys(),
            should_include_item,
            |id| Ok(FreeFunction::new(db, *id)),
        )?;
        let mut structs = filter_map_item_id_to_item(
            module_data.structs(db).keys(),
            should_include_item,
            |id| Struct::new(db, *id, include_private_items),
        )?;
        let mut enums =
            filter_map_item_id_to_item(module_data.enums(db).keys(), should_include_item, |id| {
                Enum::new(db, *id)
            })?;

        let mut type_aliases = filter_map_item_id_to_item(
            module_data.type_aliases(db).keys(),
            should_include_item,
            |id| Ok(TypeAlias::new(db, *id)),
        )?;

        let mut impl_aliases = filter_map_item_id_to_item(
            module_data.impl_aliases(db).keys(),
            should_include_item,
            |id| Ok(ImplAlias::new(db, *id)),
        )?;

        let mut traits =
            filter_map_item_id_to_item(module_data.traits(db).keys(), should_include_item, |id| {
                Trait::new(db, *id)
            })?;

        let hide_impls_for_hidden_traits =
            |impl_def_id: &&ImplDefId<'db>| is_impl_hidden(db, impl_def_id);
        let mut impls = filter_map_item_id_to_item(
            module_data
                .impls(db)
                .keys()
                .filter(hide_impls_for_hidden_traits),
            should_include_item,
            |id| Impl::new(db, *id),
        )?;
        let mut extern_types = filter_map_item_id_to_item(
            module_data.extern_types(db).keys(),
            should_include_item,
            |id| Ok(ExternType::new(db, *id)),
        )?;
        let mut extern_functions = filter_map_item_id_to_item(
            module_data.extern_functions(db).keys(),
            should_include_item,
            |id| Ok(ExternFunction::new(db, *id)),
        )?;
        let mut submodules: Vec<Module> = filter_map_item_id_to_item(
            module_data.submodules(db).keys(),
            should_include_item,
            |id| Module::new(db, ModuleId::Submodule(*id), include_private_items),
        )?;
        let mut macro_declarations: Vec<MacroDeclaration> = filter_map_item_id_to_item(
            module_data.macro_declarations(db).keys(),
            should_include_item,
            |id| Ok(MacroDeclaration::new(db, *id)),
        )?;

        let mut module_pubuses = ModulePubUses::new(db, module_id, include_private_items)?;

        let macro_mods = module_macro_modules(db, false, module_id);
        macro_mods.iter().for_each(|m_id| {
            if let Ok((
                macro_submodules,
                macro_constants,
                macro_free_functions,
                macro_structs,
                macro_enums,
                macro_type_aliases,
                macro_impl_aliases,
                macro_traits,
                macro_impls,
                macro_extern_types,
                macro_extern_functions,
                macro_macro_declarations,
                macro_pub_uses,
            )) = collect_module_items_recursive(db, *m_id, include_private_items)
            {
                submodules.extend(macro_submodules.clone());
                constants.extend(macro_constants.clone());
                free_functions.extend(macro_free_functions.clone());
                structs.extend(macro_structs.clone());
                enums.extend(macro_enums.clone());
                type_aliases.extend(macro_type_aliases.clone());
                impl_aliases.extend(macro_impl_aliases.clone());
                traits.extend(macro_traits.clone());
                impls.extend(macro_impls.clone());
                impl_aliases.extend(macro_impl_aliases.clone());
                extern_types.extend(macro_extern_types.clone());
                extern_functions.extend(macro_extern_functions.clone());
                macro_declarations.extend(macro_macro_declarations.clone());
                module_pubuses.add(macro_pub_uses);
            }
        });

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
        if !include_private_items {
            aggregate_pub_uses_groups(&module_pubuses, &mut group_map);
        }
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
            macro_declarations,
            groups,
        })
    }

    pub(crate) fn new_virtual(db: &'db ScarbDocDatabase, module_id: ModuleId<'db>) -> Self {
        let item_data = match module_id {
            ModuleId::CrateRoot(crate_id) => ItemData::new_crate(db, crate_id),
            ModuleId::Submodule(submodule_id) => ItemData::new_without_signature(
                db,
                submodule_id,
                LookupItemId::ModuleItem(ModuleItemId::Submodule(submodule_id)).into(),
            ),
            ModuleId::MacroCall { .. } => {
                todo!("TODO(#2262): Correctly handle declarative macros.")
            }
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
            macro_declarations: Default::default(),
            groups: vec![],
        }
    }

    /// Recursively traverses all the module and gets all the item [`DocumentableItemId`]s.
    pub(crate) fn get_all_item_ids(&self) -> HashMap<DocumentableItemId<'_>, &ItemData<'_>> {
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
        self.macro_declarations.iter().for_each(|item| {
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
pub(crate) fn merge_modules<'a, 'db>(
    documented_module: &'a mut Module<'db>,
    virtual_module: Module<'db>,
) -> &'a mut Module<'db> {
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
    for macro_declaration in virtual_module.macro_declarations {
        documented_module.insert_macro_declaration(macro_declaration);
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

pub(crate) fn get_ancestors_vector<'db>(
    ancestors: &mut Vec<ModuleId<'db>>,
    module_id: ModuleId<'db>,
    db: &'db ScarbDocDatabase,
) -> Vec<ModuleId<'db>> {
    match module_id {
        ModuleId::Submodule(submodule_id) => {
            ancestors.insert(0, module_id);
            let parent = submodule_id.parent_module(db);
            get_ancestors_vector(ancestors, parent, db);
        }
        ModuleId::CrateRoot(_) => {
            ancestors.insert(0, module_id);
        }
        ModuleId::MacroCall { .. } => {
            // TODO(#2262): Correctly handle declarative macros.
            ancestors.insert(0, module_id);
        }
    }
    ancestors.clone()
}

pub(crate) fn collect_pubuses<'db>(
    all_pub_uses: &mut ModulePubUses<'db>,
    module: Module<'db>,
) -> ModulePubUses<'db> {
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
    all_pub_uses
        .use_macro_declarations
        .extend(module.pub_uses.use_macro_declarations);

    for submodule in module.submodules {
        collect_pubuses(all_pub_uses, submodule);
    }
    all_pub_uses.to_owned()
}

pub(crate) fn is_doc_hidden_attr<'db>(
    db: &'db ScarbDocDatabase,
    syntax_node: &SyntaxNode<'db>,
) -> bool {
    syntax_node.has_attr_with_arg(db, "doc", "hidden")
}

fn is_public<'db>(
    db: &'db ScarbDocDatabase,
    element_id: &dyn TopLevelLanguageElementId<'db>,
) -> Maybe<bool> {
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
fn filter_map_item_id_to_item<'a, 'db, T, F, G, K>(
    items: impl Iterator<Item = &'a T>,
    should_include_item_function: F,
    generate_item_function: G,
) -> anyhow::Result<Vec<K>, DiagnosticAdded>
where
    T: 'a + Copy + TopLevelLanguageElementId<'db>,
    F: Fn(&'a dyn TopLevelLanguageElementId<'db>) -> Result<bool, DiagnosticAdded>,
    G: Fn(&'a T) -> Maybe<K>,
    'db: 'a,
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

type ModuleItems<'db> = (
    Vec<Module<'db>>,
    Vec<Constant<'db>>,
    Vec<FreeFunction<'db>>,
    Vec<Struct<'db>>,
    Vec<Enum<'db>>,
    Vec<TypeAlias<'db>>,
    Vec<ImplAlias<'db>>,
    Vec<Trait<'db>>,
    Vec<Impl<'db>>,
    Vec<ExternType<'db>>,
    Vec<ExternFunction<'db>>,
    Vec<MacroDeclaration<'db>>,
    ModulePubUses<'db>,
);

/// Used for collecting items declared within an exposed macro module.
pub fn collect_module_items_recursive<'db>(
    db: &'db ScarbDocDatabase,
    module_id: ModuleId<'db>,
    include_private_items: bool,
) -> Maybe<ModuleItems<'db>> {
    let mut constants = Vec::new();
    let mut free_functions = Vec::new();
    let mut structs = Vec::new();
    let mut enums = Vec::new();
    let mut type_aliases = Vec::new();
    let mut impl_aliases = Vec::new();
    let mut traits = Vec::new();
    let mut impls = Vec::new();
    let mut extern_types = Vec::new();
    let mut extern_functions = Vec::new();
    let mut macro_declarations = Vec::new();
    let mut submodules = Vec::new();
    let mut module_pubuses = ModulePubUses::default();

    let hide_impls_for_hidden_traits =
        |impl_def_id: &&ImplDefId<'db>| is_impl_hidden(db, impl_def_id);

    let should_include_item = |id: &dyn TopLevelLanguageElementId<'db>| {
        let syntax_node = id.stable_location(db).syntax_node(db);
        Ok((include_private_items || is_public(db, id)?) && !is_doc_hidden_attr(db, &syntax_node))
    };

    let module_data = module_id.module_data(db)?;

    constants.extend(filter_map_item_id_to_item(
        module_data.constants(db).keys(),
        should_include_item,
        |id| Ok(Constant::new(db, *id)),
    )?);

    free_functions.extend(filter_map_item_id_to_item(
        module_data.free_functions(db).keys(),
        should_include_item,
        |id| Ok(FreeFunction::new(db, *id)),
    )?);

    structs.extend(filter_map_item_id_to_item(
        module_data.structs(db).keys(),
        should_include_item,
        |id| Struct::new(db, *id, include_private_items),
    )?);

    enums.extend(filter_map_item_id_to_item(
        module_data.enums(db).keys(),
        should_include_item,
        |id| Enum::new(db, *id),
    )?);

    type_aliases.extend(filter_map_item_id_to_item(
        module_data.type_aliases(db).keys(),
        should_include_item,
        |id| Ok(TypeAlias::new(db, *id)),
    )?);

    impl_aliases.extend(filter_map_item_id_to_item(
        module_data.impl_aliases(db).keys(),
        should_include_item,
        |id| Ok(ImplAlias::new(db, *id)),
    )?);

    traits.extend(filter_map_item_id_to_item(
        module_data.traits(db).keys(),
        should_include_item,
        |id| Trait::new(db, *id),
    )?);

    impls.extend(filter_map_item_id_to_item(
        module_data
            .impls(db)
            .keys()
            .filter(hide_impls_for_hidden_traits),
        should_include_item,
        |id| Impl::new(db, *id),
    )?);

    extern_types.extend(filter_map_item_id_to_item(
        module_data.extern_types(db).keys(),
        should_include_item,
        |id| Ok(ExternType::new(db, *id)),
    )?);

    extern_functions.extend(filter_map_item_id_to_item(
        module_data.extern_functions(db).keys(),
        should_include_item,
        |id| Ok(ExternFunction::new(db, *id)),
    )?);

    submodules.extend(filter_map_item_id_to_item(
        module_data.submodules(db).keys(),
        should_include_item,
        |id| Module::new(db, ModuleId::Submodule(*id), include_private_items),
    )?);

    macro_declarations.extend(filter_map_item_id_to_item(
        module_data.macro_declarations(db).keys(),
        should_include_item,
        |id| Ok(MacroDeclaration::new(db, *id)),
    )?);

    let _module_pubuses = ModulePubUses::new(db, module_id, include_private_items)?;
    module_pubuses.add(_module_pubuses);

    let macro_mods = module_macro_modules(db, false, module_id);

    for m in macro_mods.iter() {
        let (
            mut sub_submodules,
            mut sub_constants,
            mut sub_free_functions,
            mut sub_structs,
            mut sub_enums,
            mut sub_type_aliases,
            mut sub_impl_aliases,
            mut sub_traits,
            mut sub_impls,
            mut sub_extern_types,
            mut sub_extern_functions,
            mut sub_macro_declarations,
            sub_pub_uses,
        ) = collect_module_items_recursive(db, *m, include_private_items)?;

        submodules.append(&mut sub_submodules);
        constants.append(&mut sub_constants);
        free_functions.append(&mut sub_free_functions);
        structs.append(&mut sub_structs);
        enums.append(&mut sub_enums);
        type_aliases.append(&mut sub_type_aliases);
        impl_aliases.append(&mut sub_impl_aliases);
        traits.append(&mut sub_traits);
        impls.append(&mut sub_impls);
        extern_types.append(&mut sub_extern_types);
        extern_functions.append(&mut sub_extern_functions);
        macro_declarations.append(&mut sub_macro_declarations);
        module_pubuses.add(sub_pub_uses);
    }
    Ok((
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
        macro_declarations,
        module_pubuses,
    ))
}

pub fn is_impl_hidden<'db>(db: &'db ScarbDocDatabase, impl_def_id: &ImplDefId<'db>) -> bool {
    let Ok(trait_id) = db.impl_def_trait(*impl_def_id) else {
        return true;
    };
    let Ok(item_trait) = db.module_trait_by_id(trait_id) else {
        return true;
    };

    let all_generic_args_are_hidden = db
        .impl_def_concrete_trait(*impl_def_id)
        .ok()
        .map(|concrete_trait_id| {
            let args = concrete_trait_id.generic_args(db);
            if args.is_empty() {
                return false;
            }
            args.iter()
                .filter_map(|arg_id| {
                    let GenericArgumentId::Type(type_id) = arg_id else {
                        return None;
                    };
                    let TypeLongId::Concrete(concrete_type_id) = type_id.long(db) else {
                        return None;
                    };
                    match &concrete_type_id {
                        ConcreteTypeId::Struct(struct_id) => {
                            struct_id.has_attr_with_arg(db, "doc", "hidden").ok()
                        }
                        ConcreteTypeId::Enum(enum_id) => {
                            enum_id.has_attr_with_arg(db, "doc", "hidden").ok()
                        }
                        ConcreteTypeId::Extern(extern_type_id) => {
                            extern_type_id.has_attr_with_arg(db, "doc", "hidden").ok()
                        }
                    }
                })
                .all(|x| x)
        })
        .unwrap_or(false);

    let trait_is_hidden = is_doc_hidden_attr(db, &item_trait.as_syntax_node());

    !(all_generic_args_are_hidden || trait_is_hidden)
}
