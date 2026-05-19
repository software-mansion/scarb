use anyhow::Result;
use std::error::Error;
use toml::de::Error as TomlParseError;

use scarb::core::Config;
use scarb::core::errors::{ManifestErrorWithSource, ManifestParseError};
use scarb::core::{
    MachineDiagnostic, MachineDiagnosticKind, MachineDiagnosticSeverity, MachineDiagnosticSpan,
    MachineRelatedLocation, ManifestSemanticError,
};
use scarb::ops;
use scarb_ui::OutputFormat;
use scarb_ui::components::MachineMessage;

use crate::args::MetadataArgs;

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
    let diagnostic = if let Some(sem) = error
        .chain()
        .find_map(|c| c.downcast_ref::<ManifestSemanticError>())
    {
        // Semantic validation error: ManifestErrorWithSource carries the path and source text.
        let src = error
            .chain()
            .find_map(|c| c.downcast_ref::<ManifestErrorWithSource>());
        let (span, related) = src
            .and_then(|src| {
                toml_edit::Document::parse(&src.content).ok().map(|doc| {
                    let data = sem.resolve(doc.as_table());
                    (data.span, data.related)
                })
            })
            .unwrap_or_default();
        let file = src.map(|src| src.path.to_string());
        let mut diagnostic = MachineDiagnostic::new(
            MachineDiagnosticKind::ManifestDiagnostic,
            sem.to_string(),
            MachineDiagnosticSeverity::Error,
            file.unwrap_or_else(|| "<unknown>".to_string()),
            span.map(|span| MachineDiagnosticSpan {
                start: span.start,
                end: span.end,
            })
            .unwrap_or(MachineDiagnosticSpan { start: 0, end: 0 }),
        );
        diagnostic.related = related
            .into_iter()
            .map(|related| MachineRelatedLocation {
                message: related.message,
                file: None,
                span: MachineDiagnosticSpan {
                    start: related.span.start,
                    end: related.span.end,
                },
            })
            .collect();
        diagnostic
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
            .map(|s| MachineDiagnosticSpan {
                start: s.start,
                end: s.end,
            });
        MachineDiagnostic::new(
            MachineDiagnosticKind::ManifestDiagnostic,
            message,
            MachineDiagnosticSeverity::Error,
            parse_err.path().to_string(),
            span.map(|span| MachineDiagnosticSpan {
                start: span.start,
                end: span.end,
            })
            .unwrap_or(MachineDiagnosticSpan { start: 0, end: 0 }),
        )
    } else if let Some(src) = error
        .chain()
        .find_map(|c| c.downcast_ref::<ManifestErrorWithSource>())
    {
        let message = src
            .source()
            .map(|err| err.to_string())
            .unwrap_or_else(|| error.to_string());
        MachineDiagnostic::new(
            MachineDiagnosticKind::ManifestDiagnostic,
            message,
            MachineDiagnosticSeverity::Error,
            src.path.to_string(),
            MachineDiagnosticSpan { start: 0, end: 0 },
        )
    } else {
        return;
    };

    config.ui().force_print(MachineMessage(diagnostic));
}
