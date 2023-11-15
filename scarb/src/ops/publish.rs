use anyhow::{ensure, Context, Result};
use url::Url;

use scarb_ui::components::Status;

use crate::core::{PackageId, SourceId, Workspace};
use crate::ops;
use crate::sources::RegistrySource;

pub struct PublishOpts {
    pub index_url: Url,
    pub allow_dirty: bool,
    pub verify: bool,
}

#[tracing::instrument(level = "debug", skip(opts, ws))]
pub fn publish(package_id: PackageId, opts: &PublishOpts, ws: &Workspace<'_>) -> Result<()> {
    let package = ws.fetch_package(&package_id)?.clone();

    let source_id = SourceId::for_registry(&opts.index_url)?;
    let registry_client = RegistrySource::create_client(source_id, ws.config())?;

    let supports_publish = ws
        .config()
        .tokio_handle()
        .block_on(registry_client.supports_publish())
        .with_context(|| format!("failed to check if registry supports publishing: {source_id}"))?;
    ensure!(
        supports_publish,
        "publishing packages is not supported by registry: {source_id}"
    );

    let package_opts = ops::PackageOpts {
        allow_dirty: opts.allow_dirty,
        verify: opts.verify,
    };
    let tarball = ops::package_one(package_id, &package_opts, ws)?;

    let dest_package_id = package_id.with_source_id(source_id);

    ws.config()
        .ui()
        .print(Status::new("Uploading", &dest_package_id.to_string()));

    ws.config().tokio_handle().block_on(async {
        registry_client.publish(package, tarball).await

        // TODO(mkaput): Wait for publish here.
    })?;

    Ok(())
}
