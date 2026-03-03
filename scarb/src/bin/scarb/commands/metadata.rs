use anyhow::Result;
use serde::Serialize;
use toml::de::Error as TomlParseError;

use scarb::core::Config;
use scarb::core::errors::ManifestParseError;
use scarb::ops;
use scarb_ui::OutputFormat;
use scarb_ui::components::MachineMessage;

use crate::args::MetadataArgs;

#[derive(Serialize)]
struct ManifestDiagnosticMessage {
    kind: ManifestMessageKind,
    message: String,
    file: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    span: Option<ManifestDiagnosticSpan>,
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
enum ManifestMessageKind {
    ManifestDiagnostic,
}

#[derive(Serialize)]
struct ManifestDiagnosticSpan {
    start: usize,
    end: usize,
}

#[tracing::instrument(skip_all, level = "info")]
pub fn run(args: MetadataArgs, config: &Config) -> Result<()> {
    let ws = match ops::read_workspace(config.manifest_path(), config) {
        Ok(ws) => ws,
        Err(error) => {
            emit_manifest_diagnostic_if_json(config, &error);
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

fn emit_manifest_diagnostic_if_json(config: &Config, error: &anyhow::Error) {
    if config.ui().output_format() != OutputFormat::Json {
        return;
    }

    let file = error.chain().find_map(|cause| {
        cause
            .downcast_ref::<ManifestParseError>()
            .map(|error| error.path().to_string())
    }).unwrap_or_else(|| config.manifest_path().to_string());

    let span = error
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

    let message =  error
        .chain()
        .find(|cause| cause.downcast_ref::<ManifestParseError>().is_none())
        .map(ToString::to_string)
        .unwrap_or_else(|| error.to_string());

    config
        .ui()
        .force_print(MachineMessage(ManifestDiagnosticMessage {
            kind: ManifestMessageKind::ManifestDiagnostic,
            message,
            file,
            span,
        }));
}