use crate::core::lockfile::Lockfile;
use crate::core::registry::Registry;
use crate::core::{Resolve, Summary};
use anyhow::Result;

#[tracing::instrument(level = "trace", skip_all)]
pub async fn resolve(
    _summaries: &[Summary],
    _registry: &dyn Registry,
    _lockfile: Lockfile,
) -> Result<Resolve> {
    todo!("implement")
}
