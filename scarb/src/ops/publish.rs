use anyhow::{ensure, Context, Result};
use indoc::formatdoc;
use url::Url;

use scarb_ui::components::Status;

use crate::core::registry::client::RegistryUpload;
use crate::core::{PackageId, SourceId, Workspace};
use crate::ops;
use crate::sources::RegistrySource;

use super::PackageOpts;

pub struct PublishOpts {
    pub index_url: Url,
    pub package_opts: PackageOpts,
}

#[tracing::instrument(level = "debug", skip(opts, ws))]
pub fn publish(package_id: PackageId, opts: &PublishOpts, ws: &Workspace<'_>) -> Result<()> {
    let package = ws.fetch_package(&package_id)?.clone();
    ensure!(
        package.is_publishable(),
        formatdoc! {
            r#"
                publishing disabled for package {package_id}
                help: set `publish = true` in package manifest
            "#,
            package_id = package_id
        }
    );

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

    let tarball = ops::package_one(package_id, &opts.package_opts, ws)?;

    let dest_package_id = package_id.with_source_id(source_id);

    ws.config()
        .ui()
        .print(Status::new("Uploading", &dest_package_id.to_string()));

    ws.config().tokio_handle().block_on(async {
        let upload = registry_client.publish(package, tarball).await;
        match upload {
            Ok(RegistryUpload::Success) => {
                ws.config().ui().print(Status::new(
                    "Published",
                    format!("{}", &dest_package_id).as_str(),
                ));
                Ok(())
            }
            _ => upload.map(|_| ()),
        }

        // TODO(mkaput): Wait for publish here.
    })
}
