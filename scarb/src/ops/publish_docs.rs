use anyhow::{Context, Result, ensure};
use url::Url;

use scarb_ui::components::Status;

use crate::core::registry::client::RegistryUpload;
use crate::core::{PackageId, SourceId, Workspace};
use crate::ops::docs;
use crate::sources::RegistrySource;

pub struct PublishDocsOpts {
    pub index_url: Url,
    pub force: bool,
    pub allow_dirty: bool,
}

pub fn publish_docs(
    package_id: PackageId,
    opts: &PublishDocsOpts,
    ws: &Workspace<'_>,
) -> Result<()> {
    let package = ws.fetch_package(&package_id)?.clone();
    let source_id = SourceId::for_registry(&opts.index_url)?;
    let registry_client = RegistrySource::create_client(source_id, ws.config())?;

    let supports_publish_docs = ws
        .config()
        .tokio_handle()
        .block_on(registry_client.supports_publish_docs())
        .with_context(|| {
            format!("failed to check if registry supports publishing docs: {source_id}")
        })?;

    ensure!(
        supports_publish_docs,
        "publishing docs is not supported by registry: {source_id}"
    );

    let docs_tarball = docs::package_docs_one(&package, opts.allow_dirty, ws)?;
    let dest_package_id = package_id.with_source_id(source_id);

    ws.config()
        .ui()
        .print(Status::new("Uploading", format!("docs for {}", dest_package_id).as_str(),));

    ws.config().tokio_handle().block_on(async {
        let upload_docs = registry_client
            .publish_docs(package_id, docs_tarball, opts.force)
            .await;
        match upload_docs {
            Ok(RegistryUpload::Success) => {
                ws.config().ui().print(Status::new(
                    "Published",
                    format!("docs for {}", dest_package_id).as_str(),
                ));
                Ok(())
            }
            Ok(RegistryUpload::Failure(e)) => Err(e),
            _ => upload_docs.map(|_| ()),
        }
    })
}
