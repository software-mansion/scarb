use crate::db::ScarbDocDatabase;
use crate::types::module_type::{
    Module, ModulePubUses, collect_pubuses, get_ancestors_vector, merge_modules,
};
use cairo_lang_defs::ids::{LanguageElementId, ModuleId};
use cairo_lang_diagnostics::Maybe;
use cairo_lang_filesystem::ids::CrateId;
use serde::Serialize;

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
}

impl Crate {
    pub fn new(
        db: &ScarbDocDatabase,
        crate_id: CrateId,
        include_private_items: bool,
    ) -> Maybe<Self> {
        let root_module_id = ModuleId::CrateRoot(crate_id);
        let root_module = Module::new(db, root_module_id, include_private_items)?;
        Ok(Self { root_module })
    }

    pub fn new_with_virtual_modules(
        db: &ScarbDocDatabase,
        crate_id: CrateId,
        include_private_items: bool,
    ) -> Maybe<Self> {
        let mut root = Self::new(db, crate_id, include_private_items)?;
        root.process_virtual_modules(db);
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
}
