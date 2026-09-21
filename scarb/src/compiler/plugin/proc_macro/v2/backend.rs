use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use anyhow::{Result, ensure};
use cairo_lang_defs::plugin::DynGeneratedFileAuxData;
use cairo_lang_macro::{ProcMacroResult, TextSpan, TokenStream};
use salsa::Database;
use scarb_proc_macro_host::{Expansion, ExpansionId, ProcMacroBackend};
use serde::{Deserialize, Serialize};

use crate::compiler::plugin::proc_macro::v2::aux_data::{EmittedAuxData, ProcMacroAuxData};
use crate::compiler::plugin::proc_macro::{DeclaredProcMacroInstances, ProcMacroInstance};
use crate::core::PackageId;

/// Identifies a single expansion of a procedural macro loaded from a dynamic library.
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
///
/// This is the Scarb side of [`ProcMacroBackend`]. It owns the loaded macro instances, and
/// collects the auxiliary data and full path markers that macros emit alongside the expanded
/// code, so they can be handed back to the macros in the post-processing callback.
#[derive(Debug)]
pub struct DylibBackend {
    instances: Vec<Arc<ProcMacroInstance>>,
    pub(crate) full_path_markers: RwLock<HashMap<PackageId, Vec<String>>>,
}

impl DeclaredProcMacroInstances for DylibBackend {
    fn instances(&self) -> &[Arc<ProcMacroInstance>] {
        &self.instances
    }
}

impl DylibBackend {
    pub fn try_new(macros: Vec<Arc<ProcMacroInstance>>) -> Result<Self> {
        let backend = Self {
            instances: macros,
            full_path_markers: RwLock::new(Default::default()),
        };
        // Validate expansions.
        let mut expansions = backend.expansions();
        expansions.sort_unstable_by_key(|e| (e.expansion.cairo_name.clone(), e.package_id));
        ensure!(
            expansions
                .windows(2)
                .all(|w| w[0].expansion.cairo_name != w[1].expansion.cairo_name),
            "duplicate expansions defined for procedural macros: {duplicates}",
            duplicates = expansions
                .windows(2)
                .filter(|w| w[0].expansion.cairo_name == w[1].expansion.cairo_name)
                .map(|w| format!(
                    "{} ({} and {})",
                    w[0].expansion.cairo_name.as_str(),
                    w[0].package_id,
                    w[1].package_id
                ))
                .collect::<Vec<_>>()
                .join(", ")
        );
        Ok(backend)
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

    fn expansions(&self) -> Vec<ProcMacroId> {
        self.instances
            .iter()
            .flat_map(|instance| {
                instance
                    .get_expansions()
                    .iter()
                    .map(|expansion| ProcMacroId::new(instance.package_id(), expansion.clone()))
            })
            .collect()
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

    fn on_expanded(
        &self,
        id: &ProcMacroId,
        result: &ProcMacroResult,
        aux_data: &mut EmittedAuxData,
    ) {
        // Full path markers require code modification.
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

/// The procedural macro host plugin, as used by Scarb.
pub type ProcMacroHostPlugin = scarb_proc_macro_host::ProcMacroHostPlugin<DylibBackend>;

/// The inline procedural macro plugin, as used by Scarb.
pub type ProcMacroInlinePlugin = scarb_proc_macro_host::ProcMacroInlinePlugin<DylibBackend>;
