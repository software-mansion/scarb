use std::{collections::HashMap, sync::Arc};

use scarb_proc_macro_server_types::scope::{self, ProcMacroScope};

use crate::compiler::plugin::{
    collection::WorkspaceProcMacros,
    proc_macro::{DeclaredProcMacroInstances, ExpansionQuery, ProcMacroInstance},
};

#[derive(Default)]
pub struct ProcMacroStore {
    workspace_macros: HashMap<scope::Workspace, WorkspaceProcMacros>,
}

impl ProcMacroStore {
    pub fn insert(&mut self, workspace: scope::Workspace, workspace_macros: WorkspaceProcMacros) {
        self.workspace_macros.insert(workspace, workspace_macros);
    }

    pub fn get_instance_and_hash(
        &self,
        scope: &ProcMacroScope,
        expansion: &ExpansionQuery,
    ) -> Option<(Arc<ProcMacroInstance>, u64)> {
        let ws = self.workspace_macros.get(&scope.workspace)?;
        let hosts = ws.get(&scope.component)?;
        let instance = hosts
            .iter()
            .filter_map(|plugin| plugin.find_instance_with_expansion(expansion))
            .next()?;

        Some((
            instance.clone(),
            *ws.instance_to_hash.get(&instance.package_id())?,
        ))
    }
}
