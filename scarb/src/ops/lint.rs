use std::collections::HashSet;

use crate::{
    compiler::{
        db::{build_scarb_root_database, ScarbDatabase},
        CompilationUnit, CompilationUnitAttributes,
    },
    core::{PackageId, TargetKind},
    ops,
};
use anyhow::Result;
use cairo_lang_defs::db::DefsGroup;
use cairo_lang_diagnostics::Diagnostics;
use cairo_lang_filesystem::db::FilesGroup;
use cairo_lang_filesystem::ids::CrateLongId;
use cairo_lang_semantic::diagnostic::SemanticDiagnosticKind;
use cairo_lang_semantic::{db::SemanticGroup, SemanticDiagnostic};
use cairo_lang_utils::Upcast;
use cairo_lint_core::annotate_snippets::Renderer;
use cairo_lint_core::{
    apply_file_fixes,
    diagnostics::format_diagnostic,
    get_fixes,
    plugin::{cairo_lint_plugin_suite, diagnostic_kind_from_message, CairoLintKind},
};
use itertools::Itertools;
use scarb_ui::components::Status;
use serde::Deserialize;
use smol_str::SmolStr;

use crate::core::{Package, Workspace};

use super::{compile_unit, CompilationUnitsOpts, FeaturesOpts, FeaturesSelector};

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

    // Select proc macro units that need to be compiled for Cairo compilation units.
    let required_plugins = compilation_units
        .iter()
        .flat_map(|unit| match unit {
            CompilationUnit::Cairo(unit) => unit
                .cairo_plugins
                .iter()
                .map(|p| p.package.id)
                .collect_vec(),
            _ => Vec::new(),
        })
        .collect::<HashSet<PackageId>>();

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

        // We guarantee that proc-macro units are always processed first,
        // so that all required plugins are compiled before we start checking Cairo units.
        let units = package_compilation_units.into_iter().sorted_by_key(|unit| {
            if matches!(unit, CompilationUnit::ProcMacro(_)) {
                0
            } else {
                1
            }
        });

        for compilation_unit in units {
            match compilation_unit {
                CompilationUnit::ProcMacro(_) => {
                    // We process all proc-macro units that are required by Cairo compilation units.
                    if required_plugins.contains(&compilation_unit.main_package_id()) {
                        compile_unit(compilation_unit.clone(), ws)?;
                    }
                }
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
                        let fixes = get_fixes(&db, diagnostics)?;
                        for (file_id, fixes) in fixes.into_iter() {
                            ws.config()
                                .ui()
                                .print(Status::new("Fixing", &file_id.file_name(db.upcast())));
                            apply_file_fixes(file_id, fixes, &db)?;
                        }
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
