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

macro_rules! aggregate_groups {
    ($fn_name:ident, $field:ident, $item_type:ty) => {
        pub fn $fn_name($field: &Vec<$item_type>, groups_map: &mut HashMap<String, Group>) {
            for item in $field {
                for group_name in item.item_data.groups.iter() {
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

                    group.$field.push(item.clone());
                }
            }
        }
    };
}

aggregate_groups!(aggregate_constants_groups, constants, Constant);
aggregate_groups!(
    aggregate_free_functions_by_group,
    free_functions,
    FreeFunction
);
aggregate_groups!(aggregate_structs_groups, structs, Struct);
aggregate_groups!(aggregate_enums_groups, enums, Enum);
aggregate_groups!(aggregate_type_aliases_groups, type_aliases, TypeAlias);
aggregate_groups!(aggregate_impl_aliases_groups, impl_aliases, ImplAlias);
aggregate_groups!(aggregate_traits_groups, traits, Trait);
aggregate_groups!(aggregate_impls_groups, impls, Impl);
aggregate_groups!(aggregate_extern_types_groups, extern_types, ExternType);
aggregate_groups!(
    aggregate_extern_functions_by_group,
    extern_functions,
    ExternFunction
);
aggregate_groups!(aggregate_modules_by_group, submodules, Module);
