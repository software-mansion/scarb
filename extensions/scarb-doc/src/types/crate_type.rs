use crate::db::ScarbDocDatabase;
use crate::types::groups::Group;
use crate::types::module_type::{
    Module, ModulePubUses, collect_pubuses, get_ancestors_vector, merge_modules,
};
use cairo_lang_defs::ids::{LanguageElementId, ModuleId};
use cairo_lang_diagnostics::Maybe;
use cairo_lang_doc::documentable_item::DocumentableItemId;
use cairo_lang_filesystem::ids::CrateId;
use serde::Serialize;
use std::collections::HashMap;
use std::collections::HashSet;

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

macro_rules! collect_items {
    ($group:expr, $entry:expr, $collected_ids:expr, $(($field:ident, $destination:ident)),* $(,)?) => {
        $(
            for item in $group.$field.iter() {
                if $collected_ids.insert(item.item_data.id) {
                    $entry.$destination.push(item.clone());
                }
            }
        )*
    };
}

#[derive(Serialize, Clone)]
pub struct Crate {
    pub root_module: Module,
    #[serde(skip_serializing)]
    pub foreign_crates: Vec<Module>,
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
            foreign_crates: vec![],
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
        let groups = root.collect_groups();
        root.groups = groups;

        Ok(root)
    }

    fn ensure_module_structure(
        &mut self,
        db: &ScarbDocDatabase,
        module_ids: Vec<ModuleId>,
    ) -> &mut Module {
        let mut current_module = {
            if let Some(first_ancestor) = module_ids.first() {
                if &self.root_module.module_id != first_ancestor {
                    if let Some(index) = self
                        .foreign_crates
                        .iter_mut()
                        .position(|module| module.module_id == *first_ancestor)
                    {
                        &mut self.foreign_crates[index]
                    } else {
                        self.foreign_crates
                            .push(Module::new_virtual(db, *first_ancestor));
                        self.foreign_crates.last_mut().unwrap()
                    }
                } else {
                    &mut self.root_module
                }
            } else {
                &mut self.root_module
            }
        };
        for id in module_ids.iter().skip(1) {
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
                } else if item.item_data.group.is_none() {
                    pointer.submodules.push(item);
                }
            }
        }
        self.to_owned()
    }

    pub fn collect_groups(&mut self) -> Vec<Group> {
        let mut merged_groups = HashMap::new();

        // must guarantee uniqueness of all group items
        let mut collected_ids: HashSet<DocumentableItemId> = HashSet::new();

        for module in &mut self.foreign_crates {
            Self::collect_groups_from_module(module, &mut merged_groups, &mut collected_ids);
        }
        Self::collect_groups_from_module(&self.root_module, &mut merged_groups, &mut collected_ids);

        let mut groups: Vec<Group> = merged_groups.into_values().collect();
        groups.sort_by(|a, b| a.name.cmp(&b.name));
        groups
    }

    fn collect_groups_from_module(
        module: &Module,
        merged_groups: &mut HashMap<String, Group>,
        collected_ids: &mut HashSet<DocumentableItemId>,
    ) {
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

            collect_items!(
                group,
                entry,
                collected_ids,
                (submodules, submodules),
                (constants, constants),
                (free_functions, free_functions),
                (structs, structs),
                (enums, enums),
                (type_aliases, type_aliases),
                (impl_aliases, impl_aliases),
                (traits, traits),
                (impls, impls),
                (extern_types, extern_types),
                (extern_functions, extern_functions),
            );

            for submodule in group.submodules.iter() {
                Self::collect_groups_from_module(submodule, merged_groups, collected_ids);
            }
        }

        for submodule in &module.submodules {
            Self::collect_groups_from_module(submodule, merged_groups, collected_ids);
        }
    }
}
