use std::{collections::HashMap, sync::Arc};

use scarb_proc_macro_server_types::scope::{self, ProcMacroScope};

use crate::compiler::plugin::{collection::WorkspaceProcMacros, proc_macro::ProcMacroHostPlugin};

#[derive(Default)]
pub struct ProcMacroStore {
    workspace_macros: HashMap<scope::Workspace, WorkspaceProcMacros>,
}

impl ProcMacroStore {
    pub fn insert(&mut self, workspace: scope::Workspace, workspace_macros: WorkspaceProcMacros) {
        self.workspace_macros.insert(workspace, workspace_macros);
    }

    pub fn get_plugins(&self, scope: &ProcMacroScope) -> Option<Arc<Vec<ProcMacroHostPlugin>>> {
        self.workspace_macros
            .get(&scope.workspace)?
            .get(&scope.component)
    }
}
