use crate::args::ListCommandArgs;
use anyhow::{Context, Result};
use dialoguer::console::Style;
use indoc::formatdoc;
use itertools::Itertools;
use scarb::core::ManifestDependency;
use scarb::core::registry::DEFAULT_REGISTRY_INDEX;
use scarb::core::registry::client::cache::RegistryClientCache;
use scarb::core::registry::index::IndexRecords;
use scarb::core::source::Source;
use scarb::core::{Config, DependencyVersionReq, SourceId};
use scarb::sources::{RegistrySource, StandardLibSource};
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

    let package_as_dep = ManifestDependency::builder()
        .name(package_name)
        .version_req(DependencyVersionReq::Any)
        .build();

    let std_source = StandardLibSource::new(config);
    let summaries = config
        .tokio_handle()
        .block_on(std_source.query(&package_as_dep))?;
    if !summaries.is_empty() {
        assert_eq!(
            summaries.len(),
            1,
            "Standard library should have exactly one version"
        );
        let std_version = summaries
            .iter()
            .map(|s| s.package_id.version.to_string())
            .collect::<Vec<_>>();

        config.ui().warn(
            formatdoc! {
                r#"
                the package `{package_name}` is a part of Cairo standard library.
                its available version ({version}) is coupled to the Cairo version included in your Scarb installation.
                help: to use another version of this package, consider using a different version of Scarb.
                "#,
                package_name = package_as_dep.name,
                version = std_version.first().unwrap(),
            }
        );
    }

    let index = args.index.unwrap_or(Url::from_str(DEFAULT_REGISTRY_INDEX)?);
    let source_id = SourceId::for_registry(&index)?;
    let registry_client = RegistrySource::create_client(source_id, config)?;
    let registry_client = RegistryClientCache::new(source_id, registry_client, config)?;

    let records = config
        .tokio_handle()
        .block_on(registry_client.get_records_with_cache(&package_as_dep))
        .with_context(|| {
            format!(
                "failed to lookup for `{package_as_dep}` in registry: {}",
                source_id
            )
        })?;
    let records = records
        .into_iter()
        .sorted_by_key(|r| std::cmp::Reverse(r.version.clone()))
        .collect();

    let display_limit = if args.all { None } else { Some(args.limit) };

    Ok(VersionsList {
        records,
        display_limit,
    })
}

#[derive(Serialize, Debug)]
struct VersionsList {
    records: IndexRecords,

    /// If specified, limits the number of displayed versions to this number.
    #[serde(skip)]
    display_limit: Option<usize>,
}

impl Message for VersionsList {
    fn text(self) -> String {
        use std::fmt::Write;
        let green = Style::from_dotted_str("green");
        let red = Style::from_dotted_str("red");

        let version_header = "VERSION";
        let audit_header = "AUDIT";
        let status_header = "STATUS";

        let total = self.records.len();
        let limit = self.display_limit.unwrap_or(total);

        let records = self.records.into_iter().take(limit).collect::<Vec<_>>();

        let version_width = records
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

        for record in records.into_iter() {
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

        if limit < total {
            writeln!(
                out,
                "...\nuse `--all` or `--limit {total}` to show all {total} versions"
            )
            .unwrap();
        }

        // Trim any trailing whitespace in-place.
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
