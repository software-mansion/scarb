use anyhow::Result;
use serde::Serialize;
use toml::de::Error as TomlParseError;

use scarb::core::Config;
use scarb::core::errors::ManifestParseError;
use scarb::core::{ManifestDiagnosticData, ManifestDiagnosticSpan, ManifestRelatedLocation};
use scarb::ops;
use scarb_ui::OutputFormat;
use scarb_ui::components::MachineMessage;

use crate::args::MetadataArgs;

#[derive(Serialize)]
struct ManifestDiagnosticMessage {
    kind: ManifestMessageKind,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    file: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    primary: Option<ManifestDiagnosticSpan>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    related: Vec<ManifestRelatedLocation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    span: Option<ManifestDiagnosticSpan>,
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
enum ManifestMessageKind {
    ManifestDiagnostic,
}

#[tracing::instrument(skip_all, level = "info")]
pub fn run(args: MetadataArgs, config: &Config) -> Result<()> {
    let ws = match ops::read_workspace(config.manifest_path(), config) {
        Ok(ws) => ws,
        Err(error) => {
            if config.ui().output_format() == OutputFormat::Json {
                emit_manifest_diagnostic(config, &error);
            }

            return Err(error);
        }
    };

    let features = args.features.try_into()?;
    let opts = ops::MetadataOptions {
        version: args.format_version,
        no_deps: args.no_deps,
        features,
        ignore_cairo_version: args.ignore_cairo_version,
    };

    let metadata = ops::collect_metadata(&opts, &ws)?;

    config.ui().force_print(MachineMessage(metadata));

    Ok(())
}

fn emit_manifest_diagnostic(config: &Config, error: &anyhow::Error) {
    let manifest_parse_error = error
        .chain()
        .find_map(|cause| cause.downcast_ref::<ManifestParseError>());
    let file = manifest_parse_error.map(|error| error.path().to_string());
    let typed_diagnostic = manifest_parse_error
        .and_then(ManifestParseError::diagnostic)
        .cloned();

    let parse_span = error
        .chain()
        .find_map(|cause| {
            cause
                .downcast_ref::<TomlParseError>()
                .and_then(TomlParseError::span)
        })
        .map(|span| ManifestDiagnosticSpan {
            start: span.start,
            end: span.end,
        });

    let message = error
        .chain()
        .find(|cause| cause.downcast_ref::<ManifestParseError>().is_none())
        .map(ToString::to_string)
        .unwrap_or_else(|| error.to_string());

    let (primary, related) = match typed_diagnostic {
        Some(ManifestDiagnosticData { primary, related }) => (primary, related),
        None => (None, Vec::new()),
    };

    // Some semantic manifest errors intentionally remain without an anchor/span.
    // This happens when no concrete TOML node exists to attach to, e.g.:
    // - unreadable manifest file
    // - workspace filesystem enumeration/traversal failures
    // - generated values that do not map back to a manifest node
    let span = primary.clone().or(parse_span);

    config
        .ui()
        .force_print(MachineMessage(ManifestDiagnosticMessage {
            kind: ManifestMessageKind::ManifestDiagnostic,
            message,
            file,
            primary,
            related,
            span,
        }));
}
