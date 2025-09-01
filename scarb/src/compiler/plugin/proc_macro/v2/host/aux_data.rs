use crate::compiler::plugin::proc_macro::v2::{ProcMacroHostPlugin, ProcMacroId};
use crate::core::PackageId;
use cairo_lang_defs::db::DefsGroup;
use cairo_lang_defs::plugin::GeneratedFileAuxData;
use cairo_lang_filesystem::db::FilesGroup;
use cairo_lang_macro::AuxData;
use cairo_lang_semantic::db::SemanticGroup;
use itertools::Itertools;
use scarb_stable_hash::StableHasher;
use serde::{Deserialize, Serialize};
use std::any::Any;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::vec::IntoIter;

#[derive(Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct ProcMacroAuxData {
    value: Vec<u8>,
    macro_id: ProcMacroId,
}

impl ProcMacroAuxData {
    pub fn new(value: Vec<u8>, macro_id: ProcMacroId) -> Self {
        Self { value, macro_id }
    }
}

impl From<ProcMacroAuxData> for AuxData {
    fn from(data: ProcMacroAuxData) -> Self {
        Self::new(data.value)
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EmittedAuxData(Vec<ProcMacroAuxData>);

#[typetag::serde]
impl GeneratedFileAuxData for EmittedAuxData {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn eq(&self, other: &dyn GeneratedFileAuxData) -> bool {
        self.0 == other.as_any().downcast_ref::<Self>().unwrap().0
    }

    fn hash_value(&self) -> u64 {
        let mut hasher = StableHasher::new();
        for aux_data in &self.0 {
            aux_data.hash(&mut hasher);
        }
        hasher.finish()
    }
}

impl EmittedAuxData {
    pub fn new(aux_data: ProcMacroAuxData) -> Self {
        Self(vec![aux_data])
    }

    pub fn push(&mut self, aux_data: ProcMacroAuxData) {
        self.0.push(aux_data);
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl IntoIterator for EmittedAuxData {
    type Item = ProcMacroAuxData;
    type IntoIter = IntoIter<Self::Item>;

    fn into_iter(self) -> IntoIter<ProcMacroAuxData> {
        self.0.into_iter()
    }
}

impl ProcMacroHostPlugin {
    #[tracing::instrument(level = "trace", skip_all)]
    pub(crate) fn collect_aux_data(
        &self,
        db: &dyn SemanticGroup,
    ) -> HashMap<PackageId, Vec<ProcMacroAuxData>> {
        let mut data = Vec::new();
        for crate_id in db.crates() {
            let crate_modules = db.crate_modules(*crate_id);
            for module in crate_modules.iter() {
                if let Ok(module_data) = module.module_data(db) {
                    for file_info in module_data.generated_file_aux_data(db).iter() {
                        let aux_data = file_info
                            .as_ref()
                            .and_then(|ad| ad.as_any().downcast_ref::<EmittedAuxData>());
                        if let Some(aux_data) = aux_data {
                            data.extend(aux_data.clone().into_iter());
                        }
                    }
                }
            }
        }
        data.into_iter()
            .into_group_map_by(|d| d.macro_id.package_id)
    }
}
