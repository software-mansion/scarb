pub use git::*;
pub use path::*;
pub use registry::*;
pub use standard_lib::*;

mod git;
mod path;
mod registry;
mod standard_lib;

use crate::core::{DepKind, ManifestDependency, PackageName};
use anyhow::{Context, Result, anyhow};
use indoc::formatdoc;
use std::collections::HashSet;

pub fn ensure_audit_requirement_allowed(
    dependency: &ManifestDependency,
    non_audited_whitelist: &HashSet<PackageName>,
) -> Result<()> {
    if dependency.kind != DepKind::Normal || non_audited_whitelist.contains(&dependency.name) {
        return Ok(());
    }
    let dep = dependency.name.to_string();
    let source = dependency.source_id.kind.primary_field();

    Err(anyhow!(formatdoc!(
        r#"
        help: depend on a registry package
        alternatively, consider whitelisting dependency in workspace root manifest
         --> Scarb.toml
            [workspace]
            allow-no-audits = ["{dep}"]
        "#
    )))
    .context(format!(
        "dependency `{dep}` from `{source}` source is not allowed when audit requirement is enabled"
    ))
}
