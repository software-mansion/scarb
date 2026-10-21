use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use cairo_lang_defs::plugin::DynGeneratedFileAuxData;
use cairo_lang_macro::{ProcMacroResult, TextSpan, TokenStream};
use salsa::Database;
use scarb_proc_macro_host::{Expansion, ExpansionId, ProcMacroBackend};
use serde::{Deserialize, Serialize};

use crate::compiler::plugin::proc_macro::v2::aux_data::{EmittedAuxData, ProcMacroAuxData};
use crate::compiler::plugin::proc_macro::{DeclaredProcMacroInstances, ProcMacroInstance};
use crate::core::PackageId;

/// Identifies a single expansion of a loaded procedural macro.
#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct ProcMacroId {
    pub package_id: PackageId,
    pub expansion: Expansion,
}

impl ProcMacroId {
    pub fn new(package_id: PackageId, expansion: Expansion) -> Self {
        Self {
            package_id,
            expansion,
        }
    }
}

impl ExpansionId for ProcMacroId {
    fn expansion(&self) -> &Expansion {
        &self.expansion
    }
}

/// Expands procedural macros by calling into dynamic libraries loaded in-process.
#[derive(Debug)]
pub struct DylibBackend {
    instances: Vec<Arc<ProcMacroInstance>>,
    expansions: Vec<ProcMacroId>,
    pub(crate) full_path_markers: RwLock<HashMap<PackageId, Vec<String>>>,
}

impl DeclaredProcMacroInstances for DylibBackend {
    fn instances(&self) -> &[Arc<ProcMacroInstance>] {
        &self.instances
    }
}

impl DylibBackend {
    pub fn new(macros: Vec<Arc<ProcMacroInstance>>) -> Self {
        let expansions = macros
            .iter()
            .flat_map(|instance| {
                instance
                    .get_expansions()
                    .iter()
                    .map(|expansion| ProcMacroId::new(instance.package_id(), expansion.clone()))
            })
            .collect();
        Self {
            instances: macros,
            expansions,
            full_path_markers: RwLock::new(Default::default()),
        }
    }

    pub fn instance(&self, package_id: PackageId) -> &ProcMacroInstance {
        self.instances
            .iter()
            .find(|m| m.package_id() == package_id)
            .expect("procedural macro must be registered in proc macro host")
    }
}

impl ProcMacroBackend for DylibBackend {
    type Id = ProcMacroId;
    type AuxData = EmittedAuxData;

    fn expansions(&self) -> &[ProcMacroId] {
        &self.expansions
    }

    fn expand(
        &self,
        _db: &dyn Database,
        id: &ProcMacroId,
        call_site: TextSpan,
        args: TokenStream,
        item: TokenStream,
    ) -> ProcMacroResult {
        self.instance(id.package_id)
            .try_v2()
            .expect("procedural macro using v1 api used in a context expecting v2 api")
            .generate_code(id.expansion.expansion_name.clone(), call_site, args, item)
    }

    fn expand_derives(
        &self,
        db: &dyn Database,
        derives: &[(ProcMacroId, TextSpan)],
        item: TokenStream,
    ) -> Vec<ProcMacroResult> {
        derives
            .iter()
            .map(|(id, call_site)| {
                self.expand(
                    db,
                    id,
                    call_site.clone(),
                    TokenStream::empty(),
                    item.clone(),
                )
            })
            .collect()
    }

    fn on_expanded(
        &self,
        id: &ProcMacroId,
        result: &ProcMacroResult,
        aux_data: &mut EmittedAuxData,
    ) {
        self.register_full_path_markers(id.package_id, result.full_path_markers.clone());
        if let Some(new_aux_data) = result.aux_data.clone() {
            aux_data.push(ProcMacroAuxData::new(new_aux_data.into(), id.clone()));
        }
    }

    fn finish_aux_data(&self, aux_data: EmittedAuxData) -> Option<DynGeneratedFileAuxData> {
        (!aux_data.is_empty()).then(|| DynGeneratedFileAuxData::new(aux_data))
    }

    fn doc(&self, id: &ProcMacroId) -> Option<String> {
        self.instance(id.package_id)
            .doc(id.expansion.cairo_name.clone())
    }
}
