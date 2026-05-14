use crate::compiler::helpers::{write_json, write_string};
use crate::compiler::incremental::compilation::CachedWarning;
use crate::compiler::incremental::fingerprint::UnitComponentsFingerprint;
use crate::compiler::{CairoCompilationUnit, CompilationUnitAttributes};
use crate::core::Workspace;
use crate::process::is_truthy_env;
use scarb_fs_utils as fsx;
use scarb_stable_hash::{StableHasher, short_hash};
use serde::{Deserialize, Serialize};
use std::hash::Hasher;
use std::io::BufReader;

const SCARB_CHECK_FINGERPRINT: &str = "SCARB_CHECK_FINGERPRINT";

pub struct UnitCheckFingerprint {
    unit: u64,
}

#[derive(Serialize, Deserialize)]
pub struct UnitCheckResult {
    pub warnings: Vec<CachedWarning>,
}

impl UnitCheckFingerprint {
    pub fn new(unit_fingerprint: &UnitComponentsFingerprint) -> Self {
        Self {
            unit: unit_fingerprint.digest(),
        }
    }

    pub fn compute(unit: &CairoCompilationUnit, ws: &Workspace<'_>) -> Self {
        let unit_fp = ws.config().tokio_handle().block_on(async {
            UnitComponentsFingerprint::new(unit, ws).await
        });
        Self::new(&unit_fp)
    }

    fn digest(&self) -> String {
        let mut hasher = StableHasher::new();
        hasher.write_u64(self.unit);
        hasher.finish_as_short_hash()
    }

    fn cache_filename(unit: &CairoCompilationUnit) -> String {
        let target_name = unit.main_component().target_name();
        let target_kind = unit.main_component().target_kind();
        let unit_id = short_hash((target_kind, target_name.clone()));
        format!("check-{target_name}-{unit_id}")
    }

    fn result_filename(unit: &CairoCompilationUnit) -> String {
        format!("{}.json", Self::cache_filename(unit))
    }

    pub fn is_fresh(&self, unit: &CairoCompilationUnit, ws: &Workspace<'_>) -> anyhow::Result<bool> {
        let new_digest = self.digest();
        let fingerprint_dir = unit.fingerprint_dir(ws);
        let path = fingerprint_dir.path_unchecked().join(Self::cache_filename(unit));
        if !path.exists() {
            return Ok(false);
        }
        let old_digest = fsx::read_to_string(path)?;
        Ok(new_digest == old_digest)
    }

    pub fn load_result(
        &self,
        unit: &CairoCompilationUnit,
        ws: &Workspace<'_>,
    ) -> anyhow::Result<Option<UnitCheckResult>> {
        let fingerprint_dir = unit.fingerprint_dir(ws);
        let filename = Self::result_filename(unit);
        if !fingerprint_dir.path_unchecked().join(&filename).exists() {
            return Ok(None);
        }
        let file =
            fingerprint_dir.open_ro(&filename, "unit check result file", ws.config())?;
        let result: UnitCheckResult = serde_json::from_reader(BufReader::new(&*file))?;
        Ok(Some(result))
    }

    pub fn save(
        &self,
        unit: &CairoCompilationUnit,
        warnings: Vec<CachedWarning>,
        ws: &Workspace<'_>,
    ) -> anyhow::Result<()> {
        let digest = self.digest();
        let fingerprint_dir = unit.fingerprint_dir(ws);
        write_string(
            &Self::cache_filename(unit),
            "unit check fingerprint file",
            &fingerprint_dir,
            ws,
            digest,
        )?;
        write_json(
            &Self::result_filename(unit),
            "unit check result file",
            &fingerprint_dir,
            ws,
            &UnitCheckResult { warnings },
        )?;
        Ok(())
    }
}

pub fn check_fingerprint_allowed() -> bool {
    is_truthy_env(SCARB_CHECK_FINGERPRINT, true)
}
