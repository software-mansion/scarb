use crate::db::ScarbDocDatabase;
use crate::types::groups::Group;
use crate::types::module_type::{
    Module, ModulePubUses, collect_pubuses, get_ancestors_vector, merge_modules,
};
use cairo_lang_defs::ids::{LanguageElementId, ModuleId};
use cairo_lang_diagnostics::Maybe;
use cairo_lang_filesystem::ids::CrateId;
use serde::Serialize;
use std::collections::HashMap;

macro_rules! process_virtual_module_items {
    ($all_pub_ues:expr, $self:expr, $db:expr, $(($field:ident, $insert_fn:ident)),*) => {
        $(
            for item in $all_pub_ues.$field.into_iter() {
                let parent_module_id = item.id.parent_module($db);
                let ancestors = get_ancestors_vector(&mut Vec::new(), parent_module_id, $db);
                let pointer = $self.ensure_module_structure($db, ancestors);
                pointer.$insert_fn(item);
            }
        )*
    };
}

#[derive(Serialize, Clone)]
pub struct Crate {
    pub root_module: Module,
    pub groups: Vec<Group>,
}

impl Crate {
    pub fn new(
        db: &ScarbDocDatabase,
        crate_id: CrateId,
        include_private_items: bool,
    ) -> Maybe<Self> {
        let root_module_id = ModuleId::CrateRoot(crate_id);
        let root_module = Module::new(db, root_module_id, include_private_items)?;
        Ok(Self {
            root_module,
            groups: vec![],
        })
    }

    pub fn new_with_virtual_modules_and_groups(
        db: &ScarbDocDatabase,
        crate_id: CrateId,
        include_private_items: bool,
    ) -> Maybe<Self> {
        let mut root = Self::new(db, crate_id, include_private_items)?;
        root.process_virtual_modules(db);
        let mut groups: Vec<Group> = root.collect_groups().into_values().collect();
        groups.sort_by(|a, b| a.name.cmp(&b.name));
        root.groups = groups;

        Ok(root)
    }

    fn ensure_module_structure(
        &mut self,
        db: &ScarbDocDatabase,
        module_ids: Vec<ModuleId>,
    ) -> &mut Module {
        let mut current_module = &mut self.root_module;

        for id in module_ids.iter() {
            if let Some(index) = current_module
                .submodules
                .iter()
                .position(|module| module.module_id == *id)
            {
                current_module = &mut current_module.submodules[index];
            } else {
                let new_module = Module::new_virtual(db, *id);
                current_module.submodules.push(new_module);
                let index_ = current_module.submodules.len() - 1;
                current_module = &mut current_module.submodules[index_];
            }
        }
        current_module
    }

    fn process_virtual_modules(&mut self, db: &ScarbDocDatabase) -> Self {
        let mut pub_uses = ModulePubUses::default();
        let all_pub_ues = collect_pubuses(&mut pub_uses, self.root_module.clone());

        process_virtual_module_items!(
            all_pub_ues,
            self,
            db,
            (use_constants, insert_constant),
            (use_free_functions, insert_free_function),
            (use_structs, insert_struct),
            (use_enums, insert_enum),
            (use_module_type_aliases, insert_type_alias),
            (use_impl_aliases, insert_impl_alias),
            (use_traits, insert_trait),
            (use_impl_defs, insert_impl),
            (use_extern_types, insert_extern_type),
            (use_extern_functions, insert_extern_function)
        );

        for item in all_pub_ues.use_submodules.into_iter() {
            let mut ancestors = get_ancestors_vector(&mut Vec::new(), item.module_id, db);
            if let Some(last_path) = ancestors.pop() {
                let pointer = self.ensure_module_structure(db, ancestors);
                if let Some(index) = pointer
                    .submodules
                    .iter()
                    .position(|module| module.module_id == last_path)
                {
                    merge_modules(&mut pointer.submodules[index], item);
                }
            }
        }
        self.to_owned()
    }

    pub fn collect_groups(&self) -> HashMap<String, Group> {
        let mut merged_groups = HashMap::new();
        Self::collect_groups_from_module(&self.root_module, &mut merged_groups);
        merged_groups
    }

    fn collect_groups_from_module(module: &Module, merged_groups: &mut HashMap<String, Group>) {
        for group in &module.groups {
            let entry = merged_groups
                .entry(group.name.clone())
                .or_insert_with(|| Group {
                    name: group.name.clone(),
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

            entry.submodules.extend(group.submodules.clone());
            entry.constants.extend(group.constants.clone());
            entry.free_functions.extend(group.free_functions.clone());
            entry.structs.extend(group.structs.clone());
            entry.enums.extend(group.enums.clone());
            entry.type_aliases.extend(group.type_aliases.clone());
            entry.impl_aliases.extend(group.impl_aliases.clone());
            entry.traits.extend(group.traits.clone());
            entry.impls.extend(group.impls.clone());
            entry.extern_types.extend(group.extern_types.clone());
            entry
                .extern_functions
                .extend(group.extern_functions.clone());

            for submodule in group.submodules.iter() {
                Self::collect_groups_from_module(submodule, merged_groups);
            }
        }

        for submodule in &module.submodules {
            Self::collect_groups_from_module(submodule, merged_groups);
        }
    }
}
