use crate::types::module_type::Module;
use crate::types::other_types::{
    Constant, Enum, ExternFunction, ExternType, FreeFunction, Impl, ImplAlias, Struct, Trait,
    TypeAlias,
};
use serde::Serialize;
use std::collections::HashMap;

#[derive(Serialize, Clone)]
pub struct Group {
    pub name: String,
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

impl Group {
    pub fn filename(&self) -> String {
        format!("{}.md", self.get_name_normalized())
    }
    pub fn get_name_normalized(&self) -> String {
        self.name.replace(" ", "_")
    }
}

macro_rules! aggregate_groups {
    ($fn_name:ident, $items_type:ty, $group_field:ident) => {
        pub fn $fn_name(
            items: &[$items_type],
            groups_map: &mut HashMap<String, Group>,
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

aggregate_groups!(aggregate_constants_groups, Constant, constants);
aggregate_groups!(
    aggregate_free_functions_groups,
    FreeFunction,
    free_functions
);
aggregate_groups!(aggregate_structs_groups, Struct, structs);
aggregate_groups!(aggregate_enums_groups, Enum, enums);
aggregate_groups!(aggregate_type_aliases_groups, TypeAlias, type_aliases);
aggregate_groups!(aggregate_impl_aliases_groups, ImplAlias, impl_aliases);
aggregate_groups!(aggregate_traits_groups, Trait, traits);
aggregate_groups!(aggregate_impls_groups, Impl, impls);
aggregate_groups!(aggregate_extern_types_groups, ExternType, extern_types);
aggregate_groups!(
    aggregate_extern_functions_groups,
    ExternFunction,
    extern_functions
);
aggregate_groups!(aggregate_modules_groups, Module, submodules);
