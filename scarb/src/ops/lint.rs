use std::{collections::HashSet, vec};

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
use cairo_lang_diagnostics::Diagnostics;
use cairo_lang_semantic::{SemanticDiagnostic, db::SemanticGroup};
use cairo_lang_utils::Upcast;
use cairo_lint_core::{CAIRO_LINT_TOOL_NAME, annotate_snippets::Renderer};
use cairo_lint_core::{
    CairoLintToolMetadata, apply_file_fixes, diagnostics::format_diagnostic, get_fixes,
    plugin::cairo_lint_plugin_suite,
};
use itertools::Itertools;
use scarb_ui::components::Status;

use crate::core::{Package, Workspace};

use super::{CompilationUnitsOpts, FeaturesOpts, FeaturesSelector, compile_unit};

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

    // We process all proc-macro units that are required by Cairo compilation units beforehand.
    for compilation_unit in compilation_units.iter() {
        if let CompilationUnit::ProcMacro(_) = compilation_unit {
            if required_plugins.contains(&compilation_unit.main_package_id()) {
                compile_unit(compilation_unit.clone(), ws)?;
            }
        }
    }

    for package in opts.packages {
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

                    let additional_plugins =
                        vec![cairo_lint_plugin_suite(cairo_lint_tool_metadata(&package)?)];
                    let ScarbDatabase { db, .. } =
                        build_scarb_root_database(compilation_unit, ws, additional_plugins)?;

                    let main_component = compilation_unit.main_component();
                    let crate_id = main_component.crate_id(&db);

                    let diags: Vec<Diagnostics<SemanticDiagnostic>> = db
                        .crate_modules(crate_id)
                        .iter()
                        .flat_map(|module_id| db.module_semantic_diagnostics(*module_id).ok())
                        .collect();

                    let renderer = Renderer::styled();

                    let diagnostics = diags
                        .iter()
                        .flat_map(|diags| {
                            let all_diags = diags.get_all();
                            all_diags.iter().for_each(|diag| {
                                ws.config()
                                    .ui()
                                    .print(format_diagnostic(diag, &db, &renderer))
                            });
                            all_diags
                        })
                        .collect::<Vec<_>>();

                    if opts.fix {
                        let fixes = get_fixes(&db, diagnostics);
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
