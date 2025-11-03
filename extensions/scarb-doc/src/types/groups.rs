use crate::docs_generation::markdown::get_filename_with_extension;
use crate::types::module_type::{Module, ModulePubUses};
use crate::types::other_types::{
    Constant, Enum, ExternFunction, ExternType, FreeFunction, Impl, ImplAlias, MacroDeclaration,
    Struct, Trait, TypeAlias,
};
use serde::Serialize;
use std::collections::HashMap;

#[derive(Serialize, Clone)]
pub struct Group<'db> {
    pub name: String,
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
    pub macro_declarations: Vec<MacroDeclaration<'db>>,
}

impl<'db> Group<'db> {
    pub fn filename(&self) -> String {
        get_filename_with_extension(&self.get_name_normalized())
    }
    pub fn get_name_normalized(&self) -> String {
        self.name.replace(" ", "_")
    }
}

macro_rules! aggregate_groups {
    ($fn_name:ident, $items_type:ty, $group_field:ident) => {
        pub fn $fn_name<'db>(
            items: &[$items_type],
            groups_map: &mut HashMap<String, Group<'db>>,
        ) -> Vec<$items_type> {
            let mut remaining_items = Vec::new();

            for item in items.iter() {
                if let Some(group_name) = item.item_data.group.clone() {
                    let group = groups_map
                        .entry(group_name.clone())
                        .or_insert_with(|| Group {
                            name: group_name.clone(),
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
                        });

                    group.$group_field.push(item.clone());
                } else {
                    remaining_items.push(item.clone());
                }
            }
            remaining_items
        }
    };
}

aggregate_groups!(aggregate_constants_groups, Constant<'db>, constants);
aggregate_groups!(
    aggregate_free_functions_groups,
    FreeFunction<'db>,
    free_functions
);
aggregate_groups!(aggregate_structs_groups, Struct<'db>, structs);
aggregate_groups!(aggregate_enums_groups, Enum<'db>, enums);
aggregate_groups!(aggregate_type_aliases_groups, TypeAlias<'db>, type_aliases);
aggregate_groups!(aggregate_impl_aliases_groups, ImplAlias<'db>, impl_aliases);
aggregate_groups!(aggregate_traits_groups, Trait<'db>, traits);
aggregate_groups!(aggregate_impls_groups, Impl<'db>, impls);
aggregate_groups!(aggregate_extern_types_groups, ExternType<'db>, extern_types);
aggregate_groups!(
    aggregate_extern_functions_groups,
    ExternFunction<'db>,
    extern_functions
);
aggregate_groups!(aggregate_modules_groups, Module<'db>, submodules);

pub fn aggregate_pub_uses_groups<'db>(
    module_pubuses: &ModulePubUses<'db>,
    group_map: &mut HashMap<String, Group<'db>>,
) {
    aggregate_constants_groups(&module_pubuses.use_constants, group_map);
    aggregate_free_functions_groups(&module_pubuses.use_free_functions, group_map);
    aggregate_structs_groups(&module_pubuses.use_structs, group_map);
    aggregate_enums_groups(&module_pubuses.use_enums, group_map);
    aggregate_type_aliases_groups(&module_pubuses.use_module_type_aliases, group_map);
    aggregate_impl_aliases_groups(&module_pubuses.use_impl_aliases, group_map);
    aggregate_traits_groups(&module_pubuses.use_traits, group_map);
    aggregate_impls_groups(&module_pubuses.use_impl_defs, group_map);
    aggregate_extern_types_groups(&module_pubuses.use_extern_types, group_map);
    aggregate_extern_functions_groups(&module_pubuses.use_extern_functions, group_map);
    aggregate_modules_groups(&module_pubuses.use_submodules, group_map);
}
