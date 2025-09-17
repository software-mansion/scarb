use crate::{
    DEFAULT_MODULE_MAIN_FILE,
    compiler::{
        CairoCompilationUnit, CompilationUnit, CompilationUnitAttributes, CompilationUnitComponent,
        CompilationUnitComponentId, ComponentTarget,
        db::{append_lint_plugin, build_project_config},
        plugin::collection::PluginsForComponents,
    },
    core::{PackageId, PackageName, Target, TargetKind},
    ops,
};
use salsa::Setter;
use std::{collections::HashMap, sync::Arc, vec};

use anyhow::anyhow;
use anyhow::{Context, Result};
use cairo_lang_defs::db::defs_group_input;
use cairo_lang_defs::ids::{InlineMacroExprPluginLongId, MacroPluginLongId};
use cairo_lang_defs::{db::DefsGroup, diagnostic_utils::StableLocation, ids::ModuleId};
use cairo_lang_diagnostics::{DiagnosticEntry, Severity};
use cairo_lang_filesystem::ids::CrateInput;
use cairo_lang_filesystem::{db::FilesGroup, ids::CrateLongId, override_file_content};
use cairo_lang_semantic::db::semantic_group_input;
use cairo_lang_semantic::ids::AnalyzerPluginLongId;
use cairo_lang_semantic::{
    SemanticDiagnostic, db::SemanticGroup, diagnostic::SemanticDiagnosticKind, plugin::PluginSuite,
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

    // We store the state of the workspace diagnostics, so we can decide upon throwing an error later on.
    // Also we want to apply fixes only if there were no previous errors.
    let mut packages_with_error: Vec<PackageName> = Default::default();

    let mut cus_to_lint: Vec<(&Package, &CairoCompilationUnit)> = vec![];

    for package in opts.packages.iter() {
        let package_name = &package.id.name;
        let package_compilation_units = if opts.test {
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
                ws.config()
                    .ui()
                    .print(Status::new("Skipping package", package_name.as_str()));
                continue;
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

            // If there is no compilation unit for the package, we skip it.
            match found_compilation_unit {
                Some(cu) => vec![cu],
                None => {
                    ws.config()
                        .ui()
                        .print(Status::new("Skipping package", package_name.as_str()));
                    continue;
                }
            }
        };

        let filtered_by_target_names_package_compilation_units = if opts.target_names.is_empty() {
            package_compilation_units
        } else {
            package_compilation_units
                .into_iter()
                .filter(|compilation_unit| {
                    compilation_unit
                        .main_component()
                        .targets
                        .targets()
                        .iter()
                        .any(|t| opts.target_names.contains(&t.name.to_string()))
                })
                .collect::<Vec<_>>()
        };

        cus_to_lint.extend(
            filtered_by_target_names_package_compilation_units
                .into_iter()
                .filter_map(|compilation_unit| match compilation_unit {
                    CompilationUnit::ProcMacro(_) => None,
                    CompilationUnit::Cairo(compilation_unit) => Some((package, compilation_unit)),
                })
                .collect_vec(),
        );
    }

    for (package, compilation_unit) in cus_to_lint.iter() {
        let db = build_lint_database(compilation_unit, ws)?;
        let linter_params = LinterDiagnosticParams {
            only_generated_files: false,
            tool_metadata: cairo_lint_tool_metadata(package)?,
        };
        let package_name = package.id.name.clone();
        let formatter_config = package.fmt_config()?;

        ws.config()
            .ui()
            .print(Status::new("Linting", &compilation_unit.name()));

        let main_component = compilation_unit.main_component();
        let crate_id = main_component.crate_id(&db);

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
        let diagnostics = match &absolute_path {
            Some(path) => diags
                .into_iter()
                .filter(|diag: &SemanticDiagnostic<'_>| {
                    let file_id = diag.stable_location.file_id(&db);

                    if let Ok(diag_path) = canonicalize(file_id.full_path(&db)) {
                        (path.is_dir() && diag_path.starts_with(path))
                            || (path.is_file() && diag_path == *path)
                    } else {
                        false
                    }
                })
                .collect::<Vec<_>>(),
            None => diags,
        };

        // Display diagnostics.
        for diag in &diagnostics {
            match diag.severity() {
                Severity::Error => {
                    if let Some(code) = diag.error_code() {
                        ws.config()
                            .ui()
                            .error_with_code(code.as_str(), format_diagnostic(diag, &db))
                    } else {
                        ws.config().ui().error(format_diagnostic(diag, &db))
                    }
                }
                Severity::Warning => {
                    if let Some(code) = diag.error_code() {
                        ws.config()
                            .ui()
                            .warn_with_code(code.as_str(), format_diagnostic(diag, &db))
                    } else {
                        ws.config().ui().warn(format_diagnostic(diag, &db))
                    }
                }
            }
        }

        let warnings_allowed =
            compilation_unit.compiler_config.allow_warnings && !opts.deny_warnings;

        if diagnostics.iter().any(|diag| {
            matches!(diag.severity(), Severity::Error)
                || (!warnings_allowed && matches!(diag.severity(), Severity::Warning))
        }) {
            packages_with_error.push(package_name.clone());
            continue;
        }

        if opts.fix {
            let fixes = get_fixes(&db, &linter_params, diagnostics);
            for (file_id, fixes) in fixes.into_iter() {
                ws.config()
                    .ui()
                    .print(Status::new("Fixing", &file_id.file_name(&db)));
                apply_file_fixes(file_id, fixes, &db, formatter_config.clone())?;
            }
        }
    }

    packages_with_error = packages_with_error
        .into_iter()
        .unique_by(|name| name.to_string())
        .collect();

    if !packages_with_error.is_empty() {
        if packages_with_error.len() == 1 {
            let package_name = packages_with_error[0].to_string();
            return Err(anyhow!(
                "lint checking `{package_name}` failed due to previous errors"
            ));
        } else {
            let package_names = packages_with_error
                .iter()
                .map(|name| format!("`{name}`"))
                .collect::<Vec<_>>()
                .join(", ");
            return Err(anyhow!(
                "lint checking {package_names} packages failed due to previous errors"
            ));
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

/// Keep it in sync with [crate::compiler::db::build_scarb_root_database].
fn build_lint_database(
    unit: &CairoCompilationUnit,
    ws: &Workspace<'_>,
) -> Result<LinterAnalysisDatabase> {
    let mut b = LinterAnalysisDatabase::builder();
    b.with_project_config(build_project_config(unit)?);
    b.with_cfg(unit.cfg_set.clone());

    let PluginsForComponents { mut plugins, .. } = PluginsForComponents::collect(ws, unit)?;

    append_lint_plugin(plugins.get_mut(&unit.main_component().id).unwrap());

    if !unit.compiler_config.enable_gas {
        b.skip_auto_withdraw_gas();
    }
    if unit.compiler_config.panic_backtrace {
        b.with_panic_backtrace();
    }
    if unit.compiler_config.unsafe_panic {
        b.with_unsafe_panic();
    }
    let mut db = b.build()?;

    apply_plugins(&mut db, plugins);
    inject_virtual_wrapper_lib(&mut db, unit)?;

    Ok(db)
}

/// Sets the plugin suites for crates related to the library components
/// according to the `plugins_for_components` mapping.
fn apply_plugins(
    db: &mut LinterAnalysisDatabase,
    plugins_for_components: HashMap<CompilationUnitComponentId, PluginSuite>,
) {
    for (component_id, suite) in plugins_for_components {
        let crate_input = CrateLongId::Real {
            name: component_id.cairo_package_name(),
            discriminator: component_id.to_discriminator(),
        }
        .into_crate_input(db);
        set_override_crate_plugins_from_suite(db, crate_input, suite);
    }
}

pub fn set_override_crate_plugins_from_suite(
    db: &mut LinterAnalysisDatabase,
    crate_input: CrateInput,
    plugins: PluginSuite,
) {
    let mut overrides = db.macro_plugin_overrides_input().clone();
    overrides.insert(
        crate_input.clone(),
        plugins.plugins.into_iter().map(MacroPluginLongId).collect(),
    );
    defs_group_input(db)
        .set_macro_plugin_overrides(db)
        .to(Some(overrides));

    let mut overrides = db.analyzer_plugin_overrides_input().clone();
    overrides.insert(
        crate_input.clone(),
        plugins
            .analyzer_plugins
            .into_iter()
            .map(AnalyzerPluginLongId)
            .collect(),
    );
    semantic_group_input(db)
        .set_analyzer_plugin_overrides(db)
        .to(Some(overrides));

    let mut overrides = db.inline_macro_plugin_overrides_input().clone();
    overrides.insert(
        crate_input,
        Arc::new(
            plugins
                .inline_macro_plugins
                .into_iter()
                .map(|(key, value)| (key, InlineMacroExprPluginLongId(value)))
                .collect(),
        ),
    );
    defs_group_input(db)
        .set_inline_macro_plugin_overrides(db)
        .to(Some(overrides));
}

/// Generates a wrapper lib file for appropriate compilation units.
///
/// This approach allows compiling crates that do not define `lib.cairo` file.
/// For example, single file crates can be created this way.
/// The actual single file modules are defined as `mod` items in created lib file.
fn inject_virtual_wrapper_lib(
    db: &mut LinterAnalysisDatabase,
    unit: &CairoCompilationUnit,
) -> Result<()> {
    let components: Vec<&CompilationUnitComponent> = unit
        .components
        .iter()
        .filter(|component| !component.package.id.is_core())
        // Skip components defining the default source path, as they already define lib.cairo files.
        .filter(|component| {
            let is_default_source_path = |target: &Target| {
                target
                    .source_path
                    .file_name()
                    .map(|file_name| file_name != DEFAULT_MODULE_MAIN_FILE)
                    .unwrap_or(false)
            };
            match &component.targets {
                ComponentTarget::Single(target) => is_default_source_path(target),
                ComponentTarget::Ungrouped(target) => is_default_source_path(target),
                ComponentTarget::Group(_targets) => true,
            }
        })
        .collect();

    for component in components {
        let crate_id = component.crate_id(db);

        let file_stems = component
            .targets
            .targets()
            .iter()
            .map(|target| {
                target
                    .source_path
                    .file_stem()
                    .map(|file_stem| format!("mod {file_stem};"))
                    .ok_or_else(|| {
                        anyhow!(
                            "failed to get file stem for component {}",
                            target.source_path
                        )
                    })
            })
            .collect::<Result<Vec<_>>>()?;
        let content = file_stems.join("\n");
        let module_id = ModuleId::CrateRoot(crate_id);
        let file_id = db.module_main_file(module_id).unwrap();
        // Inject virtual lib file wrapper.
        override_file_content!(db, file_id, Some(Arc::from(content.as_str())));
    }

    Ok(())
}
