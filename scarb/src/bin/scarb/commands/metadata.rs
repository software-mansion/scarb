use anyhow::Result;
use serde::Serialize;
use std::error::Error;
use toml::de::Error as TomlParseError;

use scarb::core::Config;
use scarb::core::errors::{ManifestErrorWithSource, ManifestParseError};
use scarb::core::{ManifestDiagnosticSpan, ManifestRelatedLocation, ManifestSemanticError};
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
    /// Semantic span when a typed [`ManifestSemanticError`] produced an anchor;
    /// falls back to the raw TOML parse-error span for syntax errors.
    #[serde(skip_serializing_if = "Option::is_none")]
    span: Option<ManifestDiagnosticSpan>,
    /// Related diagnostic locations for errors that span multiple TOML positions.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    related: Vec<ManifestRelatedLocation>,
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
    let (file, message, span, related) = if let Some(sem) = error
        .chain()
        .find_map(|c| c.downcast_ref::<ManifestSemanticError>())
    {
        // Semantic validation error: ManifestErrorWithSource carries the path and source text.
        let src = error
            .chain()
            .find_map(|c| c.downcast_ref::<ManifestErrorWithSource>());
        let (span, related) = src
            .map(|src| {
                let data = sem.resolve(&src.content);
                (data.span, data.related)
            })
            .unwrap_or_default();
        let file = src.map(|src| src.path.to_string());
        (file, sem.to_string(), span, related)
    } else if let Some(parse_err) = error
        .chain()
        .find_map(|c| c.downcast_ref::<ManifestParseError>())
    {
        // TOML syntax error: ManifestParseError carries the path, TomlParseError
        // carries the byte span of the offending token.
        let toml_err = error
            .chain()
            .find_map(|c| c.downcast_ref::<TomlParseError>());
        let message = if let Some(toml_err) = toml_err {
            toml_err.to_string()
        } else {
            parse_err.to_string()
        };
        let span = toml_err
            .and_then(|e| e.span())
            .map(|s| ManifestDiagnosticSpan {
                start: s.start,
                end: s.end,
            });
        (Some(parse_err.path().to_string()), message, span, vec![])
    } else if let Some(src) = error
        .chain()
        .find_map(|c| c.downcast_ref::<ManifestErrorWithSource>())
    {
        let message = src
            .source()
            .map(|err| err.to_string())
            .unwrap_or_else(|| error.to_string());
        (Some(src.path.to_string()), message, None, vec![])
    } else {
        return;
    };

    config
        .ui()
        .force_print(MachineMessage(ManifestDiagnosticMessage {
            kind: ManifestMessageKind::ManifestDiagnostic,
            message,
            file,
            span,
            related,
        }));
}
