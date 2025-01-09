use crate::{
    compiler::{
        db::{build_scarb_root_database, ScarbDatabase},
        CompilationUnit, CompilationUnitAttributes,
    },
    core::TargetKind,
    ops,
};
use annotate_snippets::Renderer;
use anyhow::{anyhow, Result};
use cairo_lang_defs::db::DefsGroup;
use cairo_lang_diagnostics::{DiagnosticEntry, Maybe};
use cairo_lang_filesystem::db::FilesGroup;
use cairo_lang_filesystem::ids::{CrateLongId, FileId};
use cairo_lang_semantic::db::SemanticGroup;
use cairo_lang_semantic::diagnostic::SemanticDiagnosticKind;
use cairo_lang_syntax::node::SyntaxNode;
use cairo_lang_utils::Upcast;
use cairo_lint_core::{
    diagnostics::format_diagnostic,
    fix::{apply_import_fixes, collect_unused_imports, fix_semantic_diagnostic, Fix, ImportFix},
    plugin::{cairo_lint_plugin_suite, diagnostic_kind_from_message, CairoLintKind},
};
use scarb_ui::components::Status;
use smol_str::SmolStr;
use std::cmp::Reverse;
use std::collections::HashMap;

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
                // TODO: Test this (if the test packages are also checked)
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
                        .print(Status::new("Checking", &compilation_unit.name()));

                    let additional_plugins = vec![cairo_lint_plugin_suite()];
                    let ScarbDatabase { db, .. } =
                        build_scarb_root_database(compilation_unit, ws, additional_plugins)?;

                    let main_component = compilation_unit
                        .components
                        .iter()
                        .find(|component| component.package.id == compilation_unit.main_package_id)
                        .expect("main component is guaranteed to exist in compilation unit");

                    let crate_id = db.intern_crate(CrateLongId::Real {
                        name: SmolStr::new(main_component.target_name()),
                        discriminator: main_component.id.to_discriminator(),
                    });
                    let mut diags = Vec::new();

                    for module_id in &*db.crate_modules(crate_id) {
                        if let Maybe::Ok(module_diags) = db.module_semantic_diagnostics(*module_id)
                        {
                            diags.push(module_diags);
                        }
                    }

                    let should_lint_panics = package
                        .tool_metadata("cairo-lint")
                        .and_then(|config| config["nopanic"].as_bool())
                        .unwrap_or(false);

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
                        // Handling unused imports separately as we need to run pre-analysis on the diagnostics.
                        // to handle complex cases.
                        let unused_imports: HashMap<FileId, HashMap<SyntaxNode, ImportFix>> =
                            collect_unused_imports(&db, &diagnostics);
                        let mut fixes = HashMap::new();
                        unused_imports.keys().for_each(|file_id| {
                            let file_fixes: Vec<Fix> =
                                apply_import_fixes(&db, unused_imports.get(file_id).unwrap());
                            fixes.insert(*file_id, file_fixes);
                        });

                        let diags_without_imports = diagnostics
                            .iter()
                            .filter(|diag| {
                                !matches!(diag.kind, SemanticDiagnosticKind::UnusedImport(_))
                            })
                            .collect::<Vec<_>>();

                        for diag in diags_without_imports {
                            if let Some((fix_node, fix)) = fix_semantic_diagnostic(&db, diag) {
                                let location = diag.location(db.upcast());
                                fixes
                                    .entry(location.file_id)
                                    .or_insert_with(Vec::new)
                                    .push(Fix {
                                        span: fix_node.span(db.upcast()),
                                        suggestion: fix,
                                    });
                            }
                        }
                        for (file_id, mut fixes) in fixes.into_iter() {
                            ws.config()
                                .ui()
                                .print(Status::new("Fixing", &file_id.file_name(db.upcast())));
                            fixes.sort_by_key(|fix| Reverse(fix.span.start));
                            let mut fixable_diagnostics = Vec::with_capacity(fixes.len());
                            if fixes.len() <= 1 {
                                fixable_diagnostics = fixes;
                            } else {
                                // Check if we have nested diagnostics. If so it's a nightmare to fix hence just ignore it
                                for i in 0..fixes.len() - 1 {
                                    let first = fixes[i].span;
                                    let second = fixes[i + 1].span;
                                    if first.start >= second.end {
                                        fixable_diagnostics.push(fixes[i].clone());
                                        if i == fixes.len() - 1 {
                                            fixable_diagnostics.push(fixes[i + 1].clone());
                                        }
                                    }
                                }
                            }
                            // Get all the files that need to be fixed
                            let mut files: HashMap<FileId, String> = HashMap::default();
                            files.insert(
                                file_id,
                                db.file_content(file_id)
                                    .ok_or(anyhow!("{} not found", file_id.file_name(db.upcast())))?
                                    .to_string(),
                            );
                            // Fix the files
                            for fix in fixable_diagnostics {
                                // Can't fail we just set the file value.
                                files.entry(file_id).and_modify(|file| {
                                    file.replace_range(fix.span.to_str_range(), &fix.suggestion)
                                });
                            }
                            // Dump them in place
                            std::fs::write(
                                file_id.full_path(db.upcast()),
                                files.get(&file_id).unwrap(),
                            )?
                        }
                    }
                }
            }
        }
    }
    Ok(())
}
