use anyhow::{Context, Result, ensure};
use indoc::formatdoc;
use url::Url;

use scarb_ui::components::Status;

use crate::core::registry::client::RegistryUpload;
use crate::core::{PackageId, SourceId, Workspace};
use crate::ops;
use crate::sources::RegistrySource;

use crate::ops::PackageOpts;
use crate::ops::PublishDocsOpts;
use crate::ops::publish_docs;

pub struct PublishOpts {
    pub index_url: Url,
    pub package_opts: PackageOpts,
    pub docs: bool,
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

    let upload_result = ws.config().tokio_handle().block_on(async {
        let upload = registry_client.publish(package, tarball).await;
        match upload {
            Ok(RegistryUpload::Success) => {
                ws.config().ui().print(Status::new(
                    "Published",
                    format!("{}", dest_package_id).as_str(),
                ));
                Ok(())
            }
            Ok(RegistryUpload::Failure(e)) => Err(e),
            _ => upload.map(|_| ()),
        }

        // TODO(mkaput): Wait for publish here.
    });

    // Upload docs if they were generated and the registry supports it.
    if opts.docs && upload_result.is_ok() {
        let docs_opts = PublishDocsOpts {
            index_url: opts.index_url.clone(),
            force: false,
            allow_dirty: opts.package_opts.allow_dirty,
        };
        let docs_result = publish_docs(package_id, &docs_opts, ws);
        if let Err(e) = docs_result {
            ws.config().ui().warn(formatdoc! {
                r#"
                    Failed to upload docs for package {package_id}: {e:?}
                    help: you can try to upload docs manually with `scarb publish-docs` or disable docs publishing with `--no-docs`
                "#,
                package_id = package_id
            });
        }
    }

    upload_result
}
