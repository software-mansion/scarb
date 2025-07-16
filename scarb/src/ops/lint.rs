use crate::compiler::{
    CairoCompilationUnit,
    db::{apply_plugins, build_project_config, inject_virtual_wrapper_lib},
    plugin::{collection::PluginsForComponents, proc_macro::ProcMacroHostPlugin},
};
use cairo_lang_filesystem::db::FilesGroupEx;
use std::{sync::Arc, vec};

use crate::{
    compiler::{CompilationUnit, CompilationUnitAttributes, db::build_scarb_root_database},
    core::{PackageId, PackageName, TargetKind},
    ops,
};

use anyhow::anyhow;
use anyhow::{Context, Result};
use cairo_lang_compiler::{
    db::{RootDatabase, validate_corelib},
    project::{ProjectConfig, update_crate_roots_from_project_config},
};
use cairo_lang_defs::{
    db::{DefsDatabase, DefsGroup, init_defs_group, try_ext_as_virtual_impl},
    diagnostic_utils::StableLocation,
};
use cairo_lang_diagnostics::{DiagnosticEntry, Severity};
use cairo_lang_filesystem::{
    cfg::CfgSet,
    db::{ExternalFiles, FilesDatabase, FilesGroup, init_dev_corelib, init_files_group},
    detect::detect_corelib,
    flag::Flag,
    ids::{FlagId, VirtualFile},
};
use cairo_lang_formatter::FormatterConfig;
use cairo_lang_lowering::{
    db::{ExternalCodeSizeEstimator, LoweringDatabase, LoweringGroup, init_lowering_group},
    utils::InliningStrategy,
};
use cairo_lang_parser::db::{ParserDatabase, ParserGroup};
use cairo_lang_semantic::{
    SemanticDiagnostic,
    db::{SemanticDatabase, SemanticGroup, init_semantic_group},
    inline_macros::get_default_plugin_suite,
    plugin::PluginSuite,
};
use cairo_lang_semantic::{db::PluginSuiteInput, diagnostic::SemanticDiagnosticKind};
use cairo_lang_sierra_generator::db::SierraGenDatabase;
use cairo_lang_syntax::node::db::{SyntaxDatabase, SyntaxGroup};
use cairo_lang_utils::Upcast;
use cairo_lint::{CAIRO_LINT_TOOL_NAME, LinterDatabase, LinterDiagnosticParams, LinterGroup};
use cairo_lint::{
    CairoLintToolMetadata, apply_file_fixes, diagnostics::format_diagnostic, get_fixes,
    plugin::cairo_lint_plugin_suite,
};
use camino::Utf8PathBuf;
use itertools::Itertools;
use scarb_ui::components::Status;

use crate::core::{Package, Workspace};
use crate::internal::fsx::canonicalize;

use super::{
    CompilationUnitsOpts, FeaturesOpts, compile_unit, plugins_required_for_units, validate_features,
};

struct CompilationUnitDiagnostics {
    pub db: ScarbDb,
    pub diagnostics: Vec<SemanticDiagnostic>,
    pub formatter_config: FormatterConfig,
}

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
        if let CompilationUnit::ProcMacro(_) = compilation_unit {
            if required_plugins.contains(&compilation_unit.main_package_id()) {
                compile_unit(compilation_unit.clone(), ws)?;
            }
        }
    }

    // We store the state of the workspace diagnostics, so we can decide upon throwing an error later on.
    // Also we want to apply fixes only if there were no previous errors.
    let mut packages_with_error: Vec<PackageName> = Default::default();
    let mut diagnostics_per_cu: Vec<CompilationUnitDiagnostics> = Default::default();

    for package in opts.packages {
        let package_name = &package.id.name;
        let formatter_config = package.fmt_config()?;
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

        for compilation_unit in filtered_by_target_names_package_compilation_units {
            match compilation_unit {
                CompilationUnit::ProcMacro(_) => {
                    continue;
                }
                CompilationUnit::Cairo(compilation_unit) => {
                    ws.config()
                        .ui()
                        .print(Status::new("Linting", &compilation_unit.name()));

                    let linter_query_params = LinterDiagnosticParams {
                        only_generated_files: false,
                        tool_metadata: cairo_lint_tool_metadata(&package)?,
                    };

                    // let additional_plugins = vec![cairo_lint_plugin_suite(
                    //     cairo_lint_tool_metadata(&package)?,
                    // )?];
                    let ScarbDatabase { db, .. } =
                        build_lint_database(compilation_unit, ws, Default::default())?;

                    let main_component = compilation_unit.main_component();
                    let crate_id = main_component.crate_id(&db);

                    // Diagnostics generated by the `cairo-lint` plugin.
                    // Only user-defined code is included, since virtual files are filtered by the `linter`.
                    let diags = db
                        .crate_modules(crate_id)
                        .iter()
                        .flat_map(|module_id| {
                            // # WAZNE!!!!
                            // TODO: Tutaj potrzebuje jeszcze semantic diagnostyk, bo fixery lintera, polegaja na diagnostykach z kompilatora o unused import warningach.
                            db.linter_diagnostics(linter_query_params.clone(), *module_id)
                        })
                        .map(|diag| {
                            SemanticDiagnostic::new(
                                StableLocation::new(diag.stable_ptr),
                                SemanticDiagnosticKind::PluginDiagnostic(diag),
                            )
                        })
                        // .flat_map(|diags| diags.get_all())
                        .collect_vec();

                    //           SemanticDiagnostic::new(
                    //     StableLocation::new(diag.stable_ptr),
                    //     SemanticDiagnosticKind::PluginDiagnostic(diag),
                    // )

                    // Filter diagnostics if `SCARB_ACTION_PATH` was provided.
                    let diagnostics = match &absolute_path {
                        Some(path) => diags
                            .into_iter()
                            .filter(|diag| {
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
                    }
                    diagnostics_per_cu.push(CompilationUnitDiagnostics {
                        db,
                        diagnostics,
                        formatter_config: formatter_config.clone(),
                    });
                }
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

    if opts.fix {
        for CompilationUnitDiagnostics {
            db,
            diagnostics,
            formatter_config,
        } in diagnostics_per_cu.into_iter()
        {
            let fixes = get_fixes(&db, diagnostics);
            for (file_id, fixes) in fixes.into_iter() {
                ws.config()
                    .ui()
                    .print(Status::new("Fixing", &file_id.file_name(&db)));
                apply_file_fixes(file_id, fixes, &db, formatter_config.clone())?;
            }
        }
    }

    Ok(())
}

fn build_lint_database(
    unit: &CairoCompilationUnit,
    ws: &Workspace<'_>,
    additional_plugins: Vec<PluginSuite>,
) -> Result<ScarbDatabase> {
    let mut b: ScarbDbBuilder = ScarbDb::builder();
    b.with_project_config(build_project_config(unit)?);
    b.with_cfg(unit.cfg_set.clone());
    b.with_inlining_strategy(unit.compiler_config.inlining_strategy.clone().into());

    let PluginsForComponents {
        mut plugins,
        proc_macros,
    } = PluginsForComponents::collect(ws, unit)?;

    // append_lint_plugin(plugins.get_mut(&unit.main_component().id).unwrap());

    let main_component_suite = plugins
        .get_mut(&unit.main_component().id)
        .expect("should be able to retrieve plugins for main component");

    for additional_suite in additional_plugins.iter() {
        main_component_suite.add(additional_suite.clone());
    }

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

    let proc_macros = proc_macros
        .into_values()
        .flat_map(|hosts| hosts.into_iter())
        .collect();
    Ok(ScarbDatabase { db, proc_macros })
}

#[salsa::database(
    LinterDatabase,
    DefsDatabase,
    FilesDatabase,
    LoweringDatabase,
    ParserDatabase,
    SemanticDatabase,
    SierraGenDatabase,
    SyntaxDatabase
)]
pub struct ScarbDb {
    storage: salsa::Storage<Self>,
}

impl ScarbDb {
    pub fn builder() -> ScarbDbBuilder {
        ScarbDbBuilder::new()
    }

    fn new(default_plugin_suite: PluginSuite, inlining_strategy: InliningStrategy) -> Self {
        let mut res = Self {
            storage: Default::default(),
        };
        init_files_group(&mut res);
        init_lowering_group(&mut res, inlining_strategy);
        init_defs_group(&mut res);
        init_semantic_group(&mut res);

        let suite = res.intern_plugin_suite(default_plugin_suite);
        res.set_default_plugins_from_suite(suite);

        res
    }
}

impl salsa::Database for ScarbDb {}
impl ExternalFiles for ScarbDb {
    fn try_ext_as_virtual(&self, external_id: salsa::InternId) -> Option<VirtualFile> {
        try_ext_as_virtual_impl(self, external_id)
    }
}

// We don't need this implementation at the moment but it's required by `LoweringGroup`.
impl ExternalCodeSizeEstimator for ScarbDb {
    fn estimate_size(
        &self,
        _function_id: cairo_lang_lowering::ids::ConcreteFunctionWithBodyId,
    ) -> cairo_lang_diagnostics::Maybe<isize> {
        cairo_lang_diagnostics::Maybe::Ok(0)
    }
}

// impl salsa::ParallelDatabase for ScarbDb {
//     fn snapshot(&self) -> salsa::Snapshot<Self> {
//         salsa::Snapshot::new(ScarbDb {
//             storage: self.storage.snapshot(),
//         })
//     }
// }

impl Upcast<dyn FilesGroup> for ScarbDb {
    fn upcast(&self) -> &(dyn FilesGroup + 'static) {
        self
    }
}

impl Upcast<dyn SyntaxGroup> for ScarbDb {
    fn upcast(&self) -> &(dyn SyntaxGroup + 'static) {
        self
    }
}

impl Upcast<dyn DefsGroup> for ScarbDb {
    fn upcast(&self) -> &(dyn DefsGroup + 'static) {
        self
    }
}

impl Upcast<dyn SemanticGroup> for ScarbDb {
    fn upcast(&self) -> &(dyn SemanticGroup + 'static) {
        self
    }
}

impl Upcast<dyn LoweringGroup> for ScarbDb {
    fn upcast(&self) -> &(dyn LoweringGroup + 'static) {
        self
    }
}

impl Upcast<dyn ParserGroup> for ScarbDb {
    fn upcast(&self) -> &(dyn ParserGroup + 'static) {
        self
    }
}

impl Upcast<dyn LinterGroup> for ScarbDb {
    fn upcast(&self) -> &(dyn LinterGroup + 'static) {
        self
    }
}

struct ScarbDatabase {
    pub db: ScarbDb,
    pub proc_macros: Vec<ProcMacroHostPlugin>,
}

#[derive(Clone, Debug)]
pub struct ScarbDbBuilder {
    default_plugin_suite: PluginSuite,
    detect_corelib: bool,
    auto_withdraw_gas: bool,
    panic_backtrace: bool,
    unsafe_panic: bool,
    project_config: Option<Box<ProjectConfig>>,
    cfg_set: Option<CfgSet>,
    inlining_strategy: InliningStrategy,
}

impl ScarbDbBuilder {
    fn new() -> Self {
        Self {
            default_plugin_suite: get_default_plugin_suite(),
            detect_corelib: false,
            auto_withdraw_gas: true,
            panic_backtrace: false,
            unsafe_panic: false,
            project_config: None,
            cfg_set: None,
            inlining_strategy: InliningStrategy::Default,
        }
    }

    pub fn with_default_plugin_suite(&mut self, suite: PluginSuite) -> &mut Self {
        self.default_plugin_suite.add(suite);
        self
    }

    pub fn clear_plugins(&mut self) -> &mut Self {
        self.default_plugin_suite = get_default_plugin_suite();
        self
    }

    pub fn with_inlining_strategy(&mut self, inlining_strategy: InliningStrategy) -> &mut Self {
        self.inlining_strategy = inlining_strategy;
        self
    }

    pub fn detect_corelib(&mut self) -> &mut Self {
        self.detect_corelib = true;
        self
    }

    pub fn with_project_config(&mut self, config: ProjectConfig) -> &mut Self {
        self.project_config = Some(Box::new(config));
        self
    }

    pub fn with_cfg(&mut self, cfg_set: impl Into<CfgSet>) -> &mut Self {
        self.cfg_set = Some(cfg_set.into());
        self
    }

    pub fn skip_auto_withdraw_gas(&mut self) -> &mut Self {
        self.auto_withdraw_gas = false;
        self
    }

    pub fn with_panic_backtrace(&mut self) -> &mut Self {
        self.panic_backtrace = true;
        self
    }

    pub fn with_unsafe_panic(&mut self) -> &mut Self {
        self.unsafe_panic = true;
        self
    }

    pub fn build(&mut self) -> Result<ScarbDb> {
        // NOTE: Order of operations matters here!
        //   Errors if something is not OK are very subtle, mostly this results in missing
        //   identifier diagnostics, or panics regarding lack of corelib items.

        let mut db = ScarbDb::new(self.default_plugin_suite.clone(), self.inlining_strategy);

        if let Some(cfg_set) = &self.cfg_set {
            db.use_cfg(cfg_set);
        }

        if self.detect_corelib {
            let path =
                detect_corelib().ok_or_else(|| anyhow!("Failed to find development corelib."))?;
            init_dev_corelib(&mut db, path)
        }

        let add_withdraw_gas_flag_id = FlagId::new(&db, "add_withdraw_gas");
        db.set_flag(
            add_withdraw_gas_flag_id,
            Some(Arc::new(Flag::AddWithdrawGas(self.auto_withdraw_gas))),
        );
        let panic_backtrace_flag_id = FlagId::new(&db, "panic_backtrace");
        db.set_flag(
            panic_backtrace_flag_id,
            Some(Arc::new(Flag::PanicBacktrace(self.panic_backtrace))),
        );

        let unsafe_panic_flag_id = FlagId::new(&db, "unsafe_panic");
        db.set_flag(
            unsafe_panic_flag_id,
            Some(Arc::new(Flag::UnsafePanic(self.unsafe_panic))),
        );

        if let Some(config) = &self.project_config {
            update_crate_roots_from_project_config(&mut db, config.as_ref());
        }
        validate_corelib(&db)?;

        Ok(db)
    }
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
