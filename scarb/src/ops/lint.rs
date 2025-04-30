use std::vec;

use crate::{
    compiler::{
        CompilationUnit, CompilationUnitAttributes,
        db::{ScarbDatabase, build_scarb_root_database},
    },
    core::{PackageId, TargetKind},
    ops,
};

use anyhow::anyhow;
use anyhow::{Context, Result};
use cairo_lang_defs::db::DefsGroup;
use cairo_lang_diagnostics::{DiagnosticEntry, Diagnostics, Severity};
use cairo_lang_semantic::{SemanticDiagnostic, db::SemanticGroup};
use cairo_lint::CAIRO_LINT_TOOL_NAME;
use cairo_lint::{
    CairoLintToolMetadata, apply_file_fixes, diagnostics::format_diagnostic, get_fixes,
    plugin::cairo_lint_plugin_suite,
};
use scarb_ui::components::Status;

use crate::core::{Package, Workspace};

use super::{
    CompilationUnitsOpts, FeaturesOpts, compile_unit, plugins_required_for_units, validate_features,
};

pub struct LintOptions {
    pub packages: Vec<Package>,
    pub test: bool,
    pub fix: bool,
    pub ignore_cairo_version: bool,
    pub features: FeaturesOpts,
    pub deny_warnings: bool,
}

#[tracing::instrument(skip_all, level = "debug")]
pub fn lint(opts: LintOptions, ws: &Workspace<'_>) -> Result<()> {
    let resolve = ops::resolve_workspace(ws)?;

    validate_features(&opts.packages, &opts.features)?;

    let compilation_units = ops::generate_compilation_units(
        &resolve,
        &opts.features,
        ws,
        CompilationUnitsOpts {
            ignore_cairo_version: opts.ignore_cairo_version,
            load_prebuilt_macros: true,
        },
    )?;

    // Select proc macro units that need to be compiled for Cairo compilation units.
    let required_plugins = plugins_required_for_units(&compilation_units);

    // We process all proc-macro units that are required by Cairo compilation units beforehand.
    for compilation_unit in compilation_units.iter() {
        if let CompilationUnit::ProcMacro(_) = compilation_unit {
            if required_plugins.contains(&compilation_unit.main_package_id()) {
                compile_unit(compilation_unit.clone(), ws)?;
            }
        }
    }

    for package in opts.packages {
        let package_name = &package.id.name;
        let package_compilation_units = if opts.test {
            let mut result = vec![];
            let integration_test_compilation_unit =
                find_integration_test_package_id(&package).map(|id| {
                    compilation_units
                        .iter()
                        .find(|compilation_unit| compilation_unit.main_package_id() == id)
                        .unwrap()
                });

            // We also want to get the main compilation unit for the package.
            if let Some(cu) = compilation_units.iter().find(|compilation_unit| {
                compilation_unit.main_package_id() == package.id
                    && compilation_unit.main_component().target_kind() != TargetKind::TEST
            }) {
                result.push(cu)
            }

            // We get all the compilation units with target kind set to "test".
            result.extend(compilation_units.iter().filter(|compilation_unit| {
                compilation_unit.main_package_id() == package.id
                    && compilation_unit.main_component().target_kind() == TargetKind::TEST
            }));

            // If any integration test compilation unit was found, we add it to the result.
            if let Some(integration_test_compilation_unit) = integration_test_compilation_unit {
                result.push(integration_test_compilation_unit);
            }

            if result.is_empty() {
                return Err(anyhow!(
                    "No Cairo compilation unit found for package {}.",
                    package.id
                ));
            }

            result
        } else {
            let found_compilation_unit =
                compilation_units
                    .iter()
                    .find(|compilation_unit| match compilation_unit {
                        CompilationUnit::Cairo(compilation_unit) => {
                            compilation_unit.main_package_id() == package.id
                                && compilation_unit.main_component().target_kind()
                                    != TargetKind::TEST
                        }
                        _ => false,
                    });
            vec![found_compilation_unit.ok_or(anyhow!(
                "No Cairo compilation unit found for package {}. Try running `--test` to include tests.",
                package.id
            ))?]
        };

        for compilation_unit in package_compilation_units {
            match compilation_unit {
                CompilationUnit::ProcMacro(_) => {
                    continue;
                }
                CompilationUnit::Cairo(compilation_unit) => {
                    ws.config()
                        .ui()
                        .print(Status::new("Linting", &compilation_unit.name()));

                    let additional_plugins = vec![cairo_lint_plugin_suite(
                        cairo_lint_tool_metadata(&package)?,
                    )?];
                    let ScarbDatabase { db, .. } =
                        build_scarb_root_database(compilation_unit, ws, additional_plugins)?;

                    let main_component = compilation_unit.main_component();
                    let crate_id = main_component.crate_id(&db);

                    let diags: Vec<Diagnostics<SemanticDiagnostic>> = db
                        .crate_modules(crate_id)
                        .iter()
                        .flat_map(|module_id| db.module_semantic_diagnostics(*module_id).ok())
                        .collect();

                    let diagnostics = diags
                        .iter()
                        .flat_map(|diags| {
                            let all_diags = diags.get_all();
                            all_diags.iter().for_each(|diag| match diag.severity() {
                                Severity::Error => {
                                    if let Some(code) = diag.error_code() {
                                        ws.config().ui().error_with_code(
                                            code.as_str(),
                                            format_diagnostic(diag, &db),
                                        )
                                    } else {
                                        ws.config().ui().error(format_diagnostic(diag, &db))
                                    }
                                }
                                Severity::Warning => {
                                    if let Some(code) = diag.error_code() {
                                        ws.config().ui().warn_with_code(
                                            code.as_str(),
                                            format_diagnostic(diag, &db),
                                        )
                                    } else {
                                        ws.config().ui().warn(format_diagnostic(diag, &db))
                                    }
                                }
                            });
                            all_diags
                        })
                        .collect::<Vec<_>>();

                    let warnings_allowed =
                        compilation_unit.compiler_config.allow_warnings && !opts.deny_warnings;

                    if diagnostics.iter().any(|diag| {
                        matches!(diag.severity(), Severity::Error)
                            || (!warnings_allowed && matches!(diag.severity(), Severity::Warning))
                    }) {
                        return Err(anyhow!(
                            "lint checking `{package_name}` failed due to previous errors"
                        ));
                    }

                    if opts.fix {
                        let fixes = get_fixes(&db, diagnostics);
                        for (file_id, fixes) in fixes.into_iter() {
                            ws.config()
                                .ui()
                                .print(Status::new("Fixing", &file_id.file_name(&db)));
                            apply_file_fixes(file_id, fixes, &db)?;
                        }
                    }
                }
            }
        }
    }
    Ok(())
}

fn cairo_lint_tool_metadata(package: &Package) -> Result<CairoLintToolMetadata> {
    Ok(package
        .tool_metadata(CAIRO_LINT_TOOL_NAME)
        .cloned()
        .map(toml::Value::try_into)
        .transpose()
        .context("Failed to parse Cairo lint tool metadata")?
        .unwrap_or_default())
}

fn find_integration_test_package_id(package: &Package) -> Option<PackageId> {
    let integration_target = package.manifest.targets.iter().find(|target| {
        target.kind == TargetKind::TEST
            && target
                .params
                .get("test-type")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                == "integration"
    });

    integration_target.map(|target| {
        package
            .id
            .for_test_target(target.group_id.clone().unwrap_or(target.name.clone()))
    })
}
