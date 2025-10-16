use crate::{
    compiler::{CairoCompilationUnit, CompilationUnit, CompilationUnitAttributes},
    core::{PackageId, PackageName, TargetKind},
    ops::{self, lint::db::build_lint_database},
};
use std::{path::PathBuf, vec};

use anyhow::anyhow;
use anyhow::{Context, Result};
use cairo_lang_defs::{db::DefsGroup, diagnostic_utils::StableLocation};
use cairo_lang_diagnostics::{DiagnosticEntry, Severity};
use cairo_lang_semantic::{
    SemanticDiagnostic, db::SemanticGroup, diagnostic::SemanticDiagnosticKind,
};
use cairo_lint::{
    CAIRO_LINT_TOOL_NAME, LinterAnalysisDatabase, LinterDiagnosticParams, LinterGroup,
};
use cairo_lint::{
    CairoLintToolMetadata, apply_file_fixes, diagnostics::format_diagnostic, get_fixes,
};
use camino::Utf8PathBuf;
use itertools::Itertools;
use scarb_ui::components::Status;

use crate::core::{Package, Workspace};
use crate::internal::fsx::canonicalize;

use super::{
    CompilationUnitsOpts, FeaturesOpts, compile_unit, plugins_required_for_units, validate_features,
};

mod db;

pub struct LintOptions {
    pub packages: Vec<Package>,
    pub target_names: Vec<String>,
    pub test: bool,
    pub fix: bool,
    pub ignore_cairo_version: bool,
    pub features: FeaturesOpts,
    pub deny_warnings: bool,
    pub path: Option<Utf8PathBuf>,
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
            load_prebuilt_macros: ws.config().load_prebuilt_proc_macros(),
        },
    )?;

    let absolute_path = opts.path.map(canonicalize).transpose()?;

    // Select proc macro units that need to be compiled for Cairo compilation units.
    let required_plugins = plugins_required_for_units(&compilation_units);

    // We process all proc-macro units that are required by Cairo compilation units beforehand.
    for compilation_unit in compilation_units.iter() {
        if let CompilationUnit::ProcMacro(_) = compilation_unit
            && required_plugins.contains(&compilation_unit.main_package_id())
        {
            compile_unit(compilation_unit.clone(), ws)?;
        }
    }

    let compilation_units_for_linting = get_compilation_units_for_linting(
        ws,
        &compilation_units,
        &opts.packages,
        opts.test,
        &opts.target_names,
    );

    // We store the state of the workspace diagnostics, so we can decide upon throwing an error later on.
    // Also we want to apply fixes only if there were no previous errors.
    let mut packages_with_error: Vec<PackageName> = Default::default();

    for (package, compilation_unit) in compilation_units_for_linting.iter() {
        ws.config()
            .ui()
            .print(Status::new("Linting", &compilation_unit.name()));

        let db = build_lint_database(compilation_unit, ws)?;
        let diagnostics = get_linting_diagnostics(&db, package, &absolute_path, compilation_unit)?;
        display_diagnostics(ws, &db, &diagnostics);

        let warnings_allowed =
            compilation_unit.compiler_config.allow_warnings && !opts.deny_warnings;

        if diagnostics.iter().any(|diag| {
            matches!(diag.severity(), Severity::Error)
                || (!warnings_allowed && matches!(diag.severity(), Severity::Warning))
        }) {
            packages_with_error.push(package.id.name.clone());
            continue;
        }

        if opts.fix {
            fix_linter_diagnostics(ws, &db, package, diagnostics)?;
        }
    }

    packages_with_error = packages_with_error
        .into_iter()
        .unique_by(|name| name.to_string())
        .collect();

    if !packages_with_error.is_empty() {
        return Err(anyhow!(get_package_linting_error_message(
            &packages_with_error
        )));
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

fn fix_linter_diagnostics(
    ws: &Workspace<'_>,
    db: &LinterAnalysisDatabase,
    package: &Package,
    diagnostics: Vec<SemanticDiagnostic<'_>>,
) -> Result<()> {
    let formatter_config = package.fmt_config()?;
    let linter_params = LinterDiagnosticParams {
        only_generated_files: false,
        tool_metadata: cairo_lint_tool_metadata(package)?,
    };
    let fixes = get_fixes(db, &linter_params, diagnostics);
    for (file_id, fixes) in fixes.into_iter() {
        ws.config()
            .ui()
            .print(Status::new("Fixing", &file_id.file_name(db).to_string(db)));
        apply_file_fixes(file_id, fixes, db, formatter_config.clone())?;
    }
    Ok(())
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

fn display_diagnostics(
    ws: &Workspace<'_>,
    db: &LinterAnalysisDatabase,
    diagnostics: &Vec<SemanticDiagnostic<'_>>,
) {
    for diag in diagnostics {
        match diag.severity() {
            Severity::Error => {
                if let Some(code) = diag.error_code() {
                    ws.config()
                        .ui()
                        .error_with_code(code.as_str(), format_diagnostic(diag, db))
                } else {
                    ws.config().ui().error(format_diagnostic(diag, db))
                }
            }
            Severity::Warning => {
                if let Some(code) = diag.error_code() {
                    ws.config()
                        .ui()
                        .warn_with_code(code.as_str(), format_diagnostic(diag, db))
                } else {
                    ws.config().ui().warn(format_diagnostic(diag, db))
                }
            }
        }
    }
}

/// Returns an error message indicating which packages failed linting due to previous errors.
fn get_package_linting_error_message(packages_with_error: &[PackageName]) -> String {
    if packages_with_error.is_empty() {
        "".to_string()
    } else if packages_with_error.len() == 1 {
        let package_name = packages_with_error[0].to_string();
        format!("lint checking `{package_name}` failed due to previous errors")
    } else {
        let package_names = packages_with_error
            .iter()
            .map(|name| format!("`{name}`"))
            .collect::<Vec<_>>()
            .join(", ");
        format!("lint checking {package_names} packages failed due to previous errors")
    }
}

fn get_linting_diagnostics<'db>(
    db: &'db LinterAnalysisDatabase,
    package: &Package,
    absolute_path: &Option<PathBuf>,
    compilation_unit: &CairoCompilationUnit,
) -> Result<Vec<SemanticDiagnostic<'db>>> {
    let linter_params = LinterDiagnosticParams {
        only_generated_files: false,
        tool_metadata: cairo_lint_tool_metadata(package)?,
    };

    let main_component = compilation_unit.main_component();
    let crate_id = main_component.crate_id(db);

    // Diagnostics generated by the `cairo-lint` plugin.
    // Only user-defined code is included, since virtual files are filtered by the `linter`.
    let diags = db
        .crate_modules(crate_id)
        .iter()
        .flat_map(|module_id| {
            let linter_diags = db
                .linter_diagnostics(linter_params.clone(), *module_id)
                .iter()
                .map(|diag| {
                    SemanticDiagnostic::new(
                        StableLocation::new(diag.stable_ptr),
                        SemanticDiagnosticKind::PluginDiagnostic(diag.clone()),
                    )
                });

            if let Ok(semantic_diags) = db.module_semantic_diagnostics(*module_id) {
                linter_diags
                    .chain(semantic_diags.get_all())
                    .collect::<Vec<_>>()
            } else {
                linter_diags.collect::<Vec<_>>()
            }
        })
        .collect_vec();

    // Filter diagnostics if `SCARB_ACTION_PATH` was provided.
    match &absolute_path {
        Some(path) => Ok(diags
            .into_iter()
            .filter(|diag: &SemanticDiagnostic<'_>| {
                let file_id = diag.stable_location.file_id(db);

                if let Ok(diag_path) = canonicalize(file_id.full_path(db)) {
                    (path.is_dir() && diag_path.starts_with(path))
                        || (path.is_file() && diag_path == *path)
                } else {
                    false
                }
            })
            .collect::<Vec<_>>()),
        None => Ok(diags),
    }
}

fn get_compilation_units_for_linting<'a>(
    ws: &Workspace<'_>,
    compilation_units: &'a Vec<CompilationUnit>,
    packages: &'a [Package],
    include_test_units: bool,
    target_names: &Vec<String>,
) -> Vec<(&'a Package, &'a CairoCompilationUnit)> {
    let mut compilation_units_for_linting: Vec<(&Package, &CairoCompilationUnit)> = vec![];

    for package in packages.iter() {
        let package_name = &package.id.name;
        let package_compilation_units = get_package_compilation_units(
            package,
            compilation_units,
            include_test_units,
            target_names,
        )
        .unwrap_or_else(|| {
            ws.config()
                .ui()
                .print(Status::new("Skipping package", package_name.as_str()));
            vec![]
        });

        compilation_units_for_linting.extend(
            package_compilation_units
                .into_iter()
                .filter_map(|compilation_unit| match compilation_unit {
                    CompilationUnit::ProcMacro(_) => None,
                    CompilationUnit::Cairo(compilation_unit) => Some((package, compilation_unit)),
                })
                .collect_vec(),
        );
    }

    compilation_units_for_linting
}

fn get_package_compilation_units<'a>(
    package: &Package,
    compilation_units: &'a [CompilationUnit],
    include_test_units: bool,
    target_names: &[String],
) -> Option<Vec<&'a CompilationUnit>> {
    let package_compilation_units = if include_test_units {
        let mut result = vec![];
        let integration_test_compilation_unit =
            find_integration_test_package_id(package).map(|id| {
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

        // If there is no compilation unit for the package, we skip it.
        if result.is_empty() {
            None
        } else {
            Some(result)
        }
    } else {
        let found_compilation_unit =
            compilation_units
                .iter()
                .find(|compilation_unit| match compilation_unit {
                    CompilationUnit::Cairo(compilation_unit) => {
                        compilation_unit.main_package_id() == package.id
                            && compilation_unit.main_component().target_kind() != TargetKind::TEST
                    }
                    _ => false,
                });

        // If there is no compilation unit for the package, we skip it.
        match found_compilation_unit {
            Some(cu) => Some(vec![cu]),
            None => None,
        }
    }?;

    if target_names.is_empty() {
        Some(package_compilation_units)
    } else {
        Some(
            package_compilation_units
                .into_iter()
                .filter(|compilation_unit| {
                    compilation_unit
                        .main_component()
                        .targets
                        .targets()
                        .iter()
                        .any(|t| target_names.contains(&t.name.to_string()))
                })
                .collect::<Vec<_>>(),
        )
    }
}
