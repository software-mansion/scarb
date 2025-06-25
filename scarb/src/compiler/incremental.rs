#![allow(dead_code)]

use crate::compiler::CairoCompilationUnit;
use std::env;

const SCARB_INCREMENTAL: &str = "SCARB_INCREMENTAL";

pub fn incremental_allowed(unit: &CairoCompilationUnit) -> bool {
    let allowed_via_env = env::var(SCARB_INCREMENTAL)
        .ok()
        .map(|var| {
            let s = var.as_str();
            s == "true" || s == "1"
        })
        .unwrap_or(true);

    let allowed_via_config = unit.compiler_config.incremental;

    allowed_via_env && allowed_via_config
}
