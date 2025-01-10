use crate::{
    compiler::{
        db::{build_scarb_root_database, ScarbDatabase},
        CompilationUnit, CompilationUnitAttributes,
    },
    core::TargetKind,
    ops,
};
use anyhow::Result;
use cairo_lang_defs::db::DefsGroup;
use cairo_lang_diagnostics::Diagnostics;
use cairo_lang_filesystem::db::FilesGroup;
use cairo_lang_filesystem::ids::CrateLongId;
use cairo_lang_semantic::diagnostic::SemanticDiagnosticKind;
use cairo_lang_semantic::{db::SemanticGroup, SemanticDiagnostic};
use cairo_lint_core::annotate_snippets::Renderer;
use cairo_lint_core::{
    apply_fixes,
    diagnostics::format_diagnostic,
    plugin::{cairo_lint_plugin_suite, diagnostic_kind_from_message, CairoLintKind},
};
use scarb_ui::components::Status;
use serde::Deserialize;
use smol_str::SmolStr;

use crate::core::{Package, Workspace};

use super::{CompilationUnitsOpts, FeaturesOpts, FeaturesSelector};

pub struct LintOptions {
    pub packages: Vec<Package>,
    pub test: bool,
    pub fix: bool,
    pub ignore_cairo_version: bool,
}

#[tracing::instrument(skip_all, level = "debug")]
pub fn lint(opts: LintOptions, ws: &Workspace<'_>) -> Result<()> {
    let feature_opts = FeaturesOpts {
        features: FeaturesSelector::AllFeatures,
        no_default_features: true,
    };

    let resolve = ops::resolve_workspace(ws)?;

    let compilation_units = ops::generate_compilation_units(
        &resolve,
        &feature_opts,
        ws,
        CompilationUnitsOpts {
            ignore_cairo_version: opts.ignore_cairo_version,
            load_prebuilt_macros: true,
        },
    )?;

    for package in opts.packages {
        let package_compilation_units = if opts.test {
            compilation_units
                .iter()
                .filter(|compilation_unit| {
                    let is_main_component = compilation_unit.main_package_id() == package.id;
                    let has_test_components =
                        compilation_unit.components().iter().any(|component| {
                            component.target_kind() == TargetKind::TEST
                                && component.package.id == package.id
                        });
                    let is_integration_test_package =
                        compilation_unit.main_package_id().name.to_string()
                            == format!("{}_integrationtest", package.id.name)
                            && compilation_unit.main_package_id().version == package.id.version;
                    is_main_component || has_test_components || is_integration_test_package
                })
                .collect::<Vec<_>>()
        } else {
            vec![compilation_units
                .iter()
                .find(|compilation_unit| compilation_unit.main_package_id() == package.id)
                .unwrap()]
        };

        for compilation_unit in package_compilation_units {
            match compilation_unit {
                // We skip proc macros as we don't want to check anything related to rust code.
                CompilationUnit::ProcMacro(_) => ws
                    .config()
                    .ui()
                    .print(Status::new("Skipping proc macro", &compilation_unit.name())),
                CompilationUnit::Cairo(compilation_unit) => {
                    ws.config()
                        .ui()
                        .print(Status::new("Linting", &compilation_unit.name()));

                    let additional_plugins = vec![cairo_lint_plugin_suite()];
                    let ScarbDatabase { db, .. } =
                        build_scarb_root_database(compilation_unit, ws, additional_plugins)?;

                    let main_component = compilation_unit.main_component();

                    let crate_id = db.intern_crate(CrateLongId::Real {
                        name: SmolStr::new(main_component.target_name()),
                        discriminator: main_component.id.to_discriminator(),
                    });

                    let diags: Vec<Diagnostics<SemanticDiagnostic>> = db
                        .crate_modules(crate_id)
                        .iter()
                        .flat_map(|module_id| db.module_semantic_diagnostics(*module_id).ok())
                        .collect();

                    let should_lint_panics = cairo_lint_tool_metadata(&package)?.nopanic;

                    let renderer = Renderer::styled();

                    let diagnostics = diags
                        .iter()
                        .flat_map(|diags| {
                            let all_diags = diags.get_all();
                            all_diags
                                .iter()
                                .filter(|diag| {
                                    if let SemanticDiagnosticKind::PluginDiagnostic(diag) =
                                        &diag.kind
                                    {
                                        (matches!(
                                            diagnostic_kind_from_message(&diag.message),
                                            CairoLintKind::Panic
                                        ) && should_lint_panics)
                                            || !matches!(
                                                diagnostic_kind_from_message(&diag.message),
                                                CairoLintKind::Panic
                                            )
                                    } else {
                                        true
                                    }
                                })
                                .for_each(|diag| {
                                    ws.config()
                                        .ui()
                                        .print(format_diagnostic(diag, &db, &renderer))
                                });
                            all_diags
                        })
                        .collect::<Vec<_>>();

                    if opts.fix {
                        let printer = |file_name: &str| {
                            ws.config().ui().print(Status::new("Fixing", file_name));
                        };
                        apply_fixes(&db, diagnostics, printer)?;
                    }
                }
            }
        }
    }
    Ok(())
}

#[derive(Deserialize, Default, Debug)]
pub struct CairoLintToolMetadata {
    pub nopanic: bool,
}

fn cairo_lint_tool_metadata(package: &Package) -> Result<CairoLintToolMetadata> {
    Ok(package
        .tool_metadata("cairo-lint")
        .cloned()
        .map(toml::Value::try_into)
        .transpose()?
        .unwrap_or_default())
}
