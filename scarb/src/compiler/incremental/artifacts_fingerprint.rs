use crate::compiler::helpers::{write_json, write_string};
use crate::compiler::incremental::fingerprint::{LocalFingerprint, UnitComponentsFingerprint};
use crate::compiler::{CairoCompilationUnit, CompilationUnitAttributes};
use crate::core::Workspace;
use crate::internal::fsx;
use itertools::Itertools;
use rayon::iter::ParallelIterator;
use rayon::prelude::IntoParallelIterator;
use scarb_stable_hash::{StableHasher, short_hash, u64_hash};
use serde::{Deserialize, Serialize};
use std::hash::{Hash, Hasher};
use std::io::BufReader;

/// Fingerprint of compiled unit artifacts.
///
/// This can be seen as a fingerprint that captures the relation between compilation inputs
/// (as defined via `UnitComponentsFingerprint`) and outputs (i.e., targets of the main component
/// of the [`CairoCompilationUnit`] and `LocalFingerprint`s of compilation artifacts).
///
/// This can be serialized to a struct that contains a list of compilation artifacts paths, which
/// should allow Scarb to recalculate the [`UnitArtifactsFingerprint`] in a following run, without
/// knowing the artifacts' locations beforehand. This works on the assumption that produced artifacts
/// are deterministic in relation to the compilation inputs and targets, thus if the artifact list
///  was bound to change, we would invalidate the fingerprint due to the first part anyway.
#[derive(Serialize, Deserialize)]
pub struct UnitArtifactsFingerprint {
    #[serde(skip)]
    pub unit: u64,
    #[serde(skip)]
    pub target: u64,
    pub local: Vec<LocalFingerprint>,
}

impl UnitArtifactsFingerprint {
    #[tracing::instrument(skip_all, level = "info")]
    pub fn new(
        unit: &CairoCompilationUnit,
        unit_fingerprint: &UnitComponentsFingerprint,
        artifacts: Vec<LocalFingerprint>,
    ) -> Self {
        UnitArtifactsFingerprint {
            unit: unit_fingerprint.digest(),
            target: u64_hash(&unit.main_component().targets),
            local: artifacts,
        }
    }

    pub fn digest(&self) -> String {
        let mut hasher = StableHasher::new();
        hasher.write_u64(self.unit);
        hasher.write_u64(self.target);
        hasher.write_usize(self.local.len());
        for local in self.local.iter().sorted_by_key(|local| local.path.clone()) {
            local.path.hash(&mut hasher);
            local.checksum.hash(&mut hasher);
        }
        hasher.finish_as_short_hash()
    }
}

#[tracing::instrument(skip_all, level = "info")]
pub fn save_unit_artifacts_fingerprint(
    unit: &CairoCompilationUnit,
    fingerprint: UnitArtifactsFingerprint,
    ws: &Workspace<'_>,
) -> anyhow::Result<()> {
    let digest = fingerprint.digest();
    let fingerprint_dir = unit.fingerprint_dir(ws);
    let target_name = unit.main_component().target_name();
    let target_kind = unit.main_component().target_kind();
    let unit_id = short_hash((target_kind, target_name.clone()));
    let filename = format!("unit-{target_name}-{unit_id}");
    write_string(
        &filename,
        "unit artifacts fingerprint digest file",
        &fingerprint_dir,
        ws,
        digest,
    )?;
    write_json(
        &format!("{filename}.json"),
        "unit artifacts fingerprint file",
        &fingerprint_dir,
        ws,
        fingerprint,
    )?;
    Ok(())
}

#[tracing::instrument(skip_all, level = "info")]
pub fn load_unit_artifacts_local_paths(
    unit: &CairoCompilationUnit,
    ws: &Workspace<'_>,
) -> anyhow::Result<Option<Vec<LocalFingerprint>>> {
    let fingerprint_dir = unit.fingerprint_dir(ws);
    let target_name = unit.main_component().target_name();
    let target_kind = unit.main_component().target_kind();
    let unit_id = short_hash((target_kind, target_name.clone()));
    let filename = format!("unit-{target_name}-{unit_id}.json");
    if !fingerprint_dir.path_unchecked().join(&filename).exists() {
        return Ok(None);
    }
    let file =
        fingerprint_dir.open_ro(&filename, "unit artifacts fingerprint file", ws.config())?;
    let file = BufReader::new(&*file);
    let artifacts_fingerprint: UnitArtifactsFingerprint = serde_json::from_reader(file)?;
    let result = artifacts_fingerprint
        .local
        .into_par_iter()
        .map(|l| {
            let content = fsx::read_to_string(&l.path)?;
            anyhow::Ok(LocalFingerprint {
                path: l.path.clone(),
                checksum: u64_hash(content),
            })
        })
        .collect::<anyhow::Result<Vec<_>>>()?;
    Ok(Some(result))
}

pub fn unit_artifacts_fingerprint_is_fresh(
    unit: &CairoCompilationUnit,
    fingerprint: UnitArtifactsFingerprint,
    ws: &Workspace<'_>,
) -> anyhow::Result<bool> {
    let new_digest = fingerprint.digest();
    let fingerprint_dir = unit.fingerprint_dir(ws);
    let target_name = unit.main_component().target_name();
    let target_kind = unit.main_component().target_kind();
    let unit_id = short_hash((target_kind, target_name.clone()));
    let filename = format!("unit-{target_name}-{unit_id}");

    let path = fingerprint_dir.path_unchecked().join(filename);
    if !path.exists() {
        return Ok(false);
    }
    let old_digest = fsx::read_to_string(path)?;

    Ok(new_digest == old_digest)
}
