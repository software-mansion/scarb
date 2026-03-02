use anyhow::Result;

use scarb::core::Config;
use scarb::ops;
use scarb_metadata as m;
use scarb_ui::components::MachineMessage;

use crate::args::MetadataArgs;

#[tracing::instrument(skip_all, level = "info")]
pub fn run(args: MetadataArgs, config: &Config) -> Result<()> {
    let validation_result = ops::validate_workspace(config.manifest_path());
    let manifest_diagnostics = validation_result
        .diagnostics
        .into_iter()
        .map(convert_manifest_diagnostic)
        .collect::<Vec<_>>();

    let metadata = if manifest_diagnostics.is_empty() {
        let ws = ops::read_workspace(config.manifest_path(), config)?;

        let features = args.features.try_into()?;
        let opts = ops::MetadataOptions {
            version: args.format_version,
            no_deps: args.no_deps,
            features,
            ignore_cairo_version: args.ignore_cairo_version,
        };

        ops::collect_metadata(&opts, &ws, manifest_diagnostics)?
    } else {
        fallback_metadata(args.format_version, config, manifest_diagnostics)?
    };

    let has_manifest_diagnostics = !metadata.manifest_diagnostics.is_empty();
    config.ui().force_print(MachineMessage(metadata));

    if has_manifest_diagnostics {
        anyhow::bail!("manifest validation failed");
    }

    Ok(())
}

fn fallback_metadata(
    format_version: u64,
    config: &Config,
    manifest_diagnostics: Vec<m::ManifestDiagnostic>,
) -> Result<m::Metadata> {
    if format_version != m::VersionPin.numeric() {
        anyhow::bail!(
            "metadata version {} not supported, only {} is currently supported",
            format_version,
            m::VersionPin
        );
    }

    let runtime_manifest = config.manifest_path().to_path_buf();
    let root = runtime_manifest
        .parent()
        .map(|path| path.to_path_buf())
        .unwrap_or_default();

    Ok(m::MetadataBuilder::default()
        .app_exe(config.app_exe().ok().map(|p| p.to_path_buf()))
        .app_version_info(ops::collect_app_version_metadata())
        .target_dir(config.target_dir_override().cloned())
        .runtime_manifest(runtime_manifest.clone())
        .workspace(
            m::WorkspaceMetadataBuilder::default()
                .manifest_path(runtime_manifest)
                .root(root)
                .members(Vec::<m::PackageId>::new())
                .build()?,
        )
        .packages(Vec::<m::PackageMetadata>::new())
        .compilation_units(Vec::<m::CompilationUnitMetadata>::new())
        .current_profile(config.profile().to_string())
        .profiles(vec!["release".to_string(), "dev".to_string()])
        .manifest_diagnostics(manifest_diagnostics)
        .build()?)
}

fn convert_manifest_diagnostic(diagnostic: ops::ManifestDiagnostic) -> m::ManifestDiagnostic {
    m::ManifestDiagnosticBuilder::default()
        .file(diagnostic.file)
        .message(diagnostic.message)
        .span(diagnostic.span.map(|span| {
            m::ManifestDiagnosticSpanBuilder::default()
                .start(span.start)
                .end(span.end)
                .build()
                .expect("manifest diagnostic span builder should be infallible")
        }))
        .build()
        .expect("manifest diagnostic builder should be infallible")
}
