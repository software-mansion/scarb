use crate::db::ScarbDocDatabase;
use crate::docs_generation::markdown::context::IncludedItems;
use crate::types::groups::{
    Group, aggregate_constants_groups, aggregate_enums_groups, aggregate_extern_functions_groups,
    aggregate_extern_types_groups, aggregate_free_functions_groups, aggregate_impl_aliases_groups,
    aggregate_impls_groups, aggregate_modules_groups, aggregate_pub_uses_groups,
    aggregate_structs_groups, aggregate_traits_groups, aggregate_type_aliases_groups,
};
use crate::types::item_data::ItemData;
use crate::types::other_types::{
    Constant, Enum, ExternFunction, ExternType, FreeFunction, Impl, ImplAlias, MacroDeclaration,
    Struct, Trait, TypeAlias,
};
use cairo_lang_defs::db::DefsGroup;
use cairo_lang_defs::ids::{
    GenericTypeId, ImplDefId, LanguageElementId, LookupItemId, ModuleId, ModuleItemId,
    NamedLanguageElementLongId, TopLevelLanguageElementId,
};
use cairo_lang_diagnostics::{DiagnosticAdded, Maybe};
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
        let Self {
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
        } = other;

        self.use_constants.extend(use_constants);
        self.use_free_functions.extend(use_free_functions);
        self.use_structs.extend(use_structs);
        self.use_enums.extend(use_enums);
        self.use_module_type_aliases.extend(use_module_type_aliases);
        self.use_impl_aliases.extend(use_impl_aliases);
        self.use_traits.extend(use_traits);
        self.use_impl_defs.extend(use_impl_defs);
        self.use_extern_types.extend(use_extern_types);
        self.use_extern_functions.extend(use_extern_functions);
        self.use_submodules.extend(use_submodules);
        self.use_macro_declarations.extend(use_macro_declarations);
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

        let mut _constants = filter_map_item_id_to_item(
            module_data.constants(db).keys(),
            should_include_item,
            |id| Ok(Constant::new(db, *id)),
        )?;
        let mut _free_functions = filter_map_item_id_to_item(
            module_data.free_functions(db).keys(),
            should_include_item,
            |id| Ok(FreeFunction::new(db, *id)),
        )?;
        let mut _structs = filter_map_item_id_to_item(
            module_data.structs(db).keys(),
            should_include_item,
            |id| Struct::new(db, *id, include_private_items),
        )?;
        let mut _enums =
            filter_map_item_id_to_item(module_data.enums(db).keys(), should_include_item, |id| {
                Enum::new(db, *id)
            })?;

        let mut _type_aliases = filter_map_item_id_to_item(
            module_data.type_aliases(db).keys(),
            should_include_item,
            |id| Ok(TypeAlias::new(db, *id)),
        )?;

        let mut _impl_aliases = filter_map_item_id_to_item(
            module_data.impl_aliases(db).keys(),
            should_include_item,
            |id| Ok(ImplAlias::new(db, *id)),
        )?;

        let mut _traits =
            filter_map_item_id_to_item(module_data.traits(db).keys(), should_include_item, |id| {
                Trait::new(db, *id)
            })?;

        let hide_impls_for_hidden_traits =
            |impl_def_id: &&ImplDefId<'db>| is_impl_hidden(db, impl_def_id);
        let mut _impls = filter_map_item_id_to_item(
            module_data
                .impls(db)
                .keys()
                .filter(hide_impls_for_hidden_traits),
            should_include_item,
            |id| Impl::new(db, *id),
        )?;
        let mut _extern_types = filter_map_item_id_to_item(
            module_data.extern_types(db).keys(),
            should_include_item,
            |id| Ok(ExternType::new(db, *id)),
        )?;
        let mut _extern_functions = filter_map_item_id_to_item(
            module_data.extern_functions(db).keys(),
            should_include_item,
            |id| Ok(ExternFunction::new(db, *id)),
        )?;
        let mut _submodules: Vec<Module> = filter_map_item_id_to_item(
            module_data.submodules(db).keys(),
            should_include_item,
            |id| Module::new(db, ModuleId::Submodule(*id), include_private_items),
        )?;
        let mut _macro_declarations: Vec<MacroDeclaration> = filter_map_item_id_to_item(
            module_data.macro_declarations(db).keys(),
            should_include_item,
            |id| Ok(MacroDeclaration::new(db, *id)),
        )?;

        let mut _module_pubuses = ModulePubUses::new(db, module_id, include_private_items)?;

        let macro_mods = module_macro_modules(db, false, module_id);
        macro_mods.iter().for_each(|m_id| {
            if let Ok(ModuleItems {
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
                pub_uses,
            }) = collect_module_items_recursive(db, *m_id, include_private_items)
            {
                _submodules.extend(submodules.clone());
                _constants.extend(constants.clone());
                _free_functions.extend(free_functions.clone());
                _structs.extend(structs.clone());
                _enums.extend(enums.clone());
                _type_aliases.extend(type_aliases.clone());
                _impl_aliases.extend(impl_aliases.clone());
                _traits.extend(traits.clone());
                _impls.extend(impls.clone());
                _impl_aliases.extend(impl_aliases.clone());
                _extern_types.extend(extern_types.clone());
                _extern_functions.extend(extern_functions.clone());
                _macro_declarations.extend(macro_declarations.clone());
                _module_pubuses.add(pub_uses);
            }
        });

        let mut group_map: HashMap<String, Group> = HashMap::new();
        _constants = aggregate_constants_groups(&_constants, &mut group_map);
        _free_functions = aggregate_free_functions_groups(&_free_functions, &mut group_map);
        _structs = aggregate_structs_groups(&_structs, &mut group_map);
        _enums = aggregate_enums_groups(&_enums, &mut group_map);
        _type_aliases = aggregate_type_aliases_groups(&_type_aliases, &mut group_map);
        _impl_aliases = aggregate_impl_aliases_groups(&_impl_aliases, &mut group_map);
        _traits = aggregate_traits_groups(&_traits, &mut group_map);
        _impls = aggregate_impls_groups(&_impls, &mut group_map);
        _extern_types = aggregate_extern_types_groups(&_extern_types, &mut group_map);
        _extern_functions = aggregate_extern_functions_groups(&_extern_functions, &mut group_map);
        _submodules = aggregate_modules_groups(&_submodules, &mut group_map);
        if !include_private_items {
            aggregate_pub_uses_groups(&_module_pubuses, &mut group_map);
        }
        let mut groups: Vec<Group> = group_map.into_values().collect();
        groups.sort_by(|a, b| a.name.cmp(&b.name));

        Ok(Self {
            module_id,
            item_data,
            submodules: _submodules,
            constants: _constants,
            free_functions: _free_functions,
            structs: _structs,
            enums: _enums,
            type_aliases: _type_aliases,
            impl_aliases: _impl_aliases,
            traits: _traits,
            impls: _impls,
            extern_types: _extern_types,
            extern_functions: _extern_functions,
            pub_uses: _module_pubuses,
            macro_declarations: _macro_declarations,
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
    pub(crate) fn get_all_item_ids<'a>(&'a self) -> IncludedItems<'a, 'db> {
        let mut ids: IncludedItems<'a, 'db> = HashMap::default();

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
            ids.extend(struct_.get_all_item_ids());
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

struct ModuleItems<'db> {
    submodules: Vec<Module<'db>>,
    constants: Vec<Constant<'db>>,
    free_functions: Vec<FreeFunction<'db>>,
    structs: Vec<Struct<'db>>,
    enums: Vec<Enum<'db>>,
    type_aliases: Vec<TypeAlias<'db>>,
    impl_aliases: Vec<ImplAlias<'db>>,
    traits: Vec<Trait<'db>>,
    impls: Vec<Impl<'db>>,
    extern_types: Vec<ExternType<'db>>,
    extern_functions: Vec<ExternFunction<'db>>,
    macro_declarations: Vec<MacroDeclaration<'db>>,
    pub_uses: ModulePubUses<'db>,
}

/// Used for collecting items declared within an exposed macro module.
fn collect_module_items_recursive<'db>(
    db: &'db ScarbDocDatabase,
    module_id: ModuleId<'db>,
    include_private_items: bool,
) -> Maybe<ModuleItems<'db>> {
    let mut _constants = Vec::new();
    let mut _free_functions = Vec::new();
    let mut _structs = Vec::new();
    let mut _enums = Vec::new();
    let mut _type_aliases = Vec::new();
    let mut _impl_aliases = Vec::new();
    let mut _traits = Vec::new();
    let mut _impls = Vec::new();
    let mut _extern_types = Vec::new();
    let mut _extern_functions = Vec::new();
    let mut _macro_declarations = Vec::new();
    let mut _submodules = Vec::new();
    let mut _module_pubuses = ModulePubUses::default();

    let hide_impls_for_hidden_traits =
        |impl_def_id: &&ImplDefId<'db>| is_impl_hidden(db, impl_def_id);

    let should_include_item = |id: &dyn TopLevelLanguageElementId<'db>| {
        let syntax_node = id.stable_location(db).syntax_node(db);
        Ok((include_private_items || is_public(db, id)?) && !is_doc_hidden_attr(db, &syntax_node))
    };

    let module_data = module_id.module_data(db)?;

    _constants.extend(filter_map_item_id_to_item(
        module_data.constants(db).keys(),
        should_include_item,
        |id| Ok(Constant::new(db, *id)),
    )?);

    _free_functions.extend(filter_map_item_id_to_item(
        module_data.free_functions(db).keys(),
        should_include_item,
        |id| Ok(FreeFunction::new(db, *id)),
    )?);

    _structs.extend(filter_map_item_id_to_item(
        module_data.structs(db).keys(),
        should_include_item,
        |id| Struct::new(db, *id, include_private_items),
    )?);

    _enums.extend(filter_map_item_id_to_item(
        module_data.enums(db).keys(),
        should_include_item,
        |id| Enum::new(db, *id),
    )?);

    _type_aliases.extend(filter_map_item_id_to_item(
        module_data.type_aliases(db).keys(),
        should_include_item,
        |id| Ok(TypeAlias::new(db, *id)),
    )?);

    _impl_aliases.extend(filter_map_item_id_to_item(
        module_data.impl_aliases(db).keys(),
        should_include_item,
        |id| Ok(ImplAlias::new(db, *id)),
    )?);

    _traits.extend(filter_map_item_id_to_item(
        module_data.traits(db).keys(),
        should_include_item,
        |id| Trait::new(db, *id),
    )?);

    _impls.extend(filter_map_item_id_to_item(
        module_data
            .impls(db)
            .keys()
            .filter(hide_impls_for_hidden_traits),
        should_include_item,
        |id| Impl::new(db, *id),
    )?);

    _extern_types.extend(filter_map_item_id_to_item(
        module_data.extern_types(db).keys(),
        should_include_item,
        |id| Ok(ExternType::new(db, *id)),
    )?);

    _extern_functions.extend(filter_map_item_id_to_item(
        module_data.extern_functions(db).keys(),
        should_include_item,
        |id| Ok(ExternFunction::new(db, *id)),
    )?);

    _submodules.extend(filter_map_item_id_to_item(
        module_data.submodules(db).keys(),
        should_include_item,
        |id| Module::new(db, ModuleId::Submodule(*id), include_private_items),
    )?);

    _macro_declarations.extend(filter_map_item_id_to_item(
        module_data.macro_declarations(db).keys(),
        should_include_item,
        |id| Ok(MacroDeclaration::new(db, *id)),
    )?);

    let module_pubuses = ModulePubUses::new(db, module_id, include_private_items)?;
    _module_pubuses.add(module_pubuses);

    let macro_mods = module_macro_modules(db, false, module_id);

    for m in macro_mods.iter() {
        let ModuleItems {
            mut submodules,
            mut constants,
            mut free_functions,
            mut structs,
            mut enums,
            mut type_aliases,
            mut impl_aliases,
            mut traits,
            mut impls,
            mut extern_types,
            mut extern_functions,
            mut macro_declarations,
            pub_uses,
        } = collect_module_items_recursive(db, *m, include_private_items)?;

        _submodules.append(&mut submodules);
        _constants.append(&mut constants);
        _free_functions.append(&mut free_functions);
        _structs.append(&mut structs);
        _enums.append(&mut enums);
        _type_aliases.append(&mut type_aliases);
        _impl_aliases.append(&mut impl_aliases);
        _traits.append(&mut traits);
        _impls.append(&mut impls);
        _extern_types.append(&mut extern_types);
        _extern_functions.append(&mut extern_functions);
        _macro_declarations.append(&mut macro_declarations);
        _module_pubuses.add(pub_uses);
    }
    Ok(ModuleItems {
        submodules: _submodules,
        constants: _constants,
        free_functions: _free_functions,
        structs: _structs,
        enums: _enums,
        type_aliases: _type_aliases,
        impl_aliases: _impl_aliases,
        traits: _traits,
        impls: _impls,
        extern_types: _extern_types,
        extern_functions: _extern_functions,
        macro_declarations: _macro_declarations,
        pub_uses: _module_pubuses,
    })
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
