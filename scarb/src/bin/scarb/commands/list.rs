use crate::args::ListCommandArgs;
use anyhow::{Context, Result};
use dialoguer::console::Style;
use itertools::Itertools;
use scarb::core::ManifestDependency;
use scarb::core::registry::DEFAULT_REGISTRY_INDEX;
use scarb::core::registry::client::cache::RegistryClientCache;
use scarb::core::registry::index::IndexRecords;
use scarb::core::{Config, DependencyVersionReq, SourceId};
use scarb::sources::RegistrySource;
use scarb_ui::Message;
use serde::{Serialize, Serializer};
use std::collections::BTreeMap;
use std::str::FromStr;
use url::Url;

#[tracing::instrument(skip_all, level = "info")]
pub fn run(args: ListCommandArgs, config: &Config) -> Result<()> {
    list_versions(args, config)
}

fn list_versions(args: ListCommandArgs, config: &Config) -> Result<()> {
    let package_name = args.package_name;
    let index = args.index.unwrap_or(Url::from_str(DEFAULT_REGISTRY_INDEX)?);

    let source_id = SourceId::for_registry(&index)?;
    let registry_client = RegistrySource::create_client(source_id, config)?;
    let registry_client = RegistryClientCache::new(source_id, registry_client, config)?;

    let dependency = ManifestDependency::builder()
        .name(package_name)
        .version_req(DependencyVersionReq::Any)
        .build();
    let records = config
        .tokio_handle()
        .block_on(registry_client.get_records_with_cache(&dependency))
        .with_context(|| {
            format!(
                "failed to lookup for `{dependency}` in registry: {}",
                source_id
            )
        })?;
    let list = VersionsList { versions: records };
    config.ui().print(list);
    Ok(())
}

#[derive(Serialize, Debug)]
struct VersionsList {
    versions: IndexRecords,
}

impl Message for VersionsList {
    fn text(self) -> String {
        self.versions
            .into_iter()
            .rev()
            .map(|record| {
                if record.yanked {
                    Style::from_dotted_str("red")
                        .apply_to(format!("{} (yanked)", record.version))
                        .to_string()
                } else {
                    record.version.to_string()
                }
            })
            .join("\n")
    }

    fn structured<S: Serializer>(self, ser: S) -> Result<S::Ok, S::Error> {
        self.versions.serialize(ser)
    }
}
