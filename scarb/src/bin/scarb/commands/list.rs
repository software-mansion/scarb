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
use std::str::FromStr;
use url::Url;

#[tracing::instrument(skip_all, level = "info")]
pub fn run(args: ListCommandArgs, config: &Config) -> Result<()> {
    config.ui().print(list_versions(args, config)?);
    Ok(())
}

fn list_versions(args: ListCommandArgs, config: &Config) -> Result<VersionsList> {
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
    let records = records
        .into_iter()
        .sorted_by_key(|r| std::cmp::Reverse(r.version.clone()))
        .collect();

    Ok(VersionsList { records })
}

#[derive(Serialize, Debug)]
struct VersionsList {
    records: IndexRecords,
}

impl Message for VersionsList {
    fn text(self) -> String {
        use std::fmt::Write;
        let green = Style::from_dotted_str("green");
        let red = Style::from_dotted_str("red");

        let version_header = "VERSION";
        let audit_header = "AUDIT";
        let status_header = "STATUS";

        let version_width = self
            .records
            .iter()
            .map(|r| r.version.to_string().len())
            .max()
            .unwrap_or(version_header.len())
            .max(version_header.len());

        let mut out = String::new();
        let gap = "    ";
        writeln!(
            out,
            "{:<version_width$}{gap}{:<5}{gap}{:<6}",
            version_header, audit_header, status_header,
        )
        .unwrap();

        for record in self.records.into_iter() {
            let (audit, audit_styled) = if record.audited {
                let text = "✓";
                (text, green.apply_to(text).to_string())
            } else {
                let text = "x";
                (text, red.apply_to(text).to_string())
            };
            let (status, status_styled) = if record.yanked {
                let text = "yanked";
                (text, red.apply_to(text).to_string())
            } else {
                let text = "-";
                (text, text.to_string())
            };

            writeln!(
                out,
                "{:<version_width$}{gap}{}{gap}{}",
                record.version.to_string(),
                pad_styled_left(audit_styled, audit, 5),
                pad_styled_left(status_styled, status, 6),
            )
            .unwrap();
        }

        out.truncate(out.trim_end().len());
        out
    }

    fn structured<S: Serializer>(self, ser: S) -> Result<S::Ok, S::Error> {
        self.records.serialize(ser)
    }
}

// Since styled text may contain ANSI escape codes, its length may differ from the length of the raw text.
// This function adds padding to the right of the styled text based on the length of the raw text and desired column width.
fn pad_styled_left(styled_text: String, raw_text: &str, column_width: usize) -> String {
    let raw_width = raw_text.chars().count();
    let pad_width = column_width.saturating_sub(raw_width);
    format!("{styled_text}{}", " ".repeat(pad_width))
}
