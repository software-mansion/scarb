use crate::compiler::db::{build_scarb_root_database, ScarbDatabase};
use crate::compiler::helpers::{build_compiler_config, write_string};
use crate::compiler::{CairoCompilationUnit, CompilationUnit, CompilationUnitAttributes};
use crate::core::{Package, TargetKind, Workspace};
use crate::ops;
use crate::ops::FeaturesOpts;
use anyhow::{anyhow, bail, Context, Result};
use cairo_lang_compiler::db::RootDatabase;
use cairo_lang_compiler::diagnostics::DiagnosticsError;
use cairo_lang_defs::db::DefsGroup;
use cairo_lang_defs::ids::{LanguageElementId, ModuleId, ModuleItemId};
use cairo_lang_defs::patcher::PatchBuilder;
use cairo_lang_diagnostics::ToOption;
use cairo_lang_filesystem::db::FilesGroup;
use cairo_lang_filesystem::ids::CrateLongId;
use cairo_lang_formatter::{CairoFormatter, FormatOutcome, FormatterConfig};
use cairo_lang_parser::db::ParserGroup;
use cairo_lang_syntax::node::helpers::UsePathEx;
use cairo_lang_syntax::node::{ast, TypedStablePtr, TypedSyntaxNode};
use cairo_lang_utils::Upcast;
use std::collections::HashSet;

#[derive(Debug)]
pub struct ExpandOpts {
    pub features: FeaturesOpts,
    pub ugly: bool,
}

pub fn expand(package: Package, opts: ExpandOpts, ws: &Workspace<'_>) -> Result<()> {
    let package_name = package.id.name.to_string();
    let resolve = ops::resolve_workspace(ws)?;
    let compilation_units = ops::generate_compilation_units(&resolve, &opts.features, ws)?;

    // Compile procedural macros.
    compilation_units
        .iter()
        .filter(|unit| matches!(unit, CompilationUnit::ProcMacro(_)))
        .map(|unit| ops::compile::compile_unit(unit.clone(), ws))
        .collect::<Result<Vec<_>>>()?;

    let Some(compilation_unit) = compilation_units.into_iter().find(|unit| {
        unit.main_package_id() == package.id
            && unit.main_component().target_kind() == TargetKind::LIB
    }) else {
        bail!("compilation unit not found for `{package_name}`")
    };
    let CompilationUnit::Cairo(compilation_unit) = compilation_unit else {
        bail!("only cairo compilation units can be expanded")
    };
    let ScarbDatabase { db, .. } = build_scarb_root_database(&compilation_unit, ws)?;
    let mut compiler_config = build_compiler_config(&compilation_unit, ws);
    compiler_config
        .diagnostics_reporter
        .ensure(&db)
        .map_err(|err| err.into())
        .map_err(|err| {
            if !suppress_error(&err) {
                ws.config().ui().anyhow(&err);
            }

            anyhow!("could not check `{package_name}` due to previous error")
        })?;

    do_expand(&db, &compilation_unit, opts, ws)?;

    Ok(())
}

/// Memorize opened modules for adding appropriate bracketing to the code.
struct ModuleStack(Vec<String>);

impl ModuleStack {
    fn new() -> Self {
        Self(Vec::new())
    }

    /// Register a module path in the stack, opening new module blocks if necessary.
    fn register(&mut self, module_path: String) -> String {
        let open_module = |builder: &mut Vec<String>| {
            let module = module_path
                .split("::")
                .last()
                .expect("module full path cannot be empty");
            builder.push(format!("\nmod {module} {{\n"));
        };
        let close_module = |builder: &mut Vec<String>| {
            builder.push(" }\n".to_string());
        };
        let mut builder: Vec<String> = Vec::new();
        while !self.0.is_empty() {
            // Can safely unwrap, as the stack is not empty.
            let current_module = self.0.last().unwrap();
            if current_module.clone() != module_path {
                if module_path.starts_with(current_module) {
                    self.0.push(module_path.clone());
                    open_module(&mut builder);
                    break;
                } else {
                    close_module(&mut builder);
                    self.0.pop();
                    continue;
                }
            } else {
                break;
            }
        }
        if self.0.is_empty() {
            self.0.push(module_path.clone());
            open_module(&mut builder);
        }
        builder.concat()
    }

    /// Pop all module paths from the stack, closing all module blocks.
    fn drain(&mut self) -> String {
        let mut builder = String::new();
        while !self.0.is_empty() {
            builder = format!("{builder}}}\n");
            self.0.pop();
        }
        builder
    }
}

fn do_expand(
    db: &RootDatabase,
    compilation_unit: &CairoCompilationUnit,
    opts: ExpandOpts,
    ws: &Workspace<'_>,
) -> Result<()> {
    let main_crate_id = db.intern_crate(CrateLongId::Real(
        compilation_unit.main_component().cairo_package_name(),
    ));
    let main_module = ModuleId::CrateRoot(main_crate_id);
    let module_file = db
        .module_main_file(main_module)
        .to_option()
        .context("failed to retrieve module main file")?;
    let file_syntax = db
        .file_module_syntax(module_file)
        .to_option()
        .context("failed to retrieve module main file syntax")?;

    let crate_modules = db.crate_modules(main_crate_id);
    let item_asts = file_syntax.items(db);

    let mut builder = PatchBuilder::new(db, &item_asts);
    let mut module_stack = ModuleStack::new();

    for module_id in crate_modules.iter() {
        builder.add_str(module_stack.register(module_id.full_path(db)).as_str());
        let Some(module_items) = db.module_items(*module_id).to_option() else {
            continue;
        };
        let mut seen_uses = HashSet::new();
        for item_id in module_items.iter() {
            // We need to handle uses manually, as module data only includes use leaf instead of path.
            if let ModuleItemId::Use(use_id) = item_id {
                let use_item = use_id.stable_ptr(db).lookup(db.upcast());
                let item = ast::UsePath::Leaf(use_item.clone()).get_item(db.upcast());
                let item = item.use_path(db.upcast());
                // We need to deduplicate multi-uses (`a::{b, c}`), which are split into multiple leaves.
                if !seen_uses.insert(item.stable_ptr()) {
                    continue;
                }
                builder.add_str("use ");
                builder.add_node(item.as_syntax_node());
                builder.add_str(";\n");
                continue;
            }
            // We can skip submodules, as they will be printed as part of `crate_modules`.
            if let ModuleItemId::Submodule(_) = item_id {
                continue;
            }
            let node = item_id.stable_location(db).syntax_node(db);
            builder.add_node(node);
        }
    }

    builder.add_str(module_stack.drain().as_str());
    let (content, _) = builder.build();
    let content = if opts.ugly {
        content
    } else {
        // Ignores formatting errors.
        format_cairo(content.clone()).unwrap_or(content)
    };

    let file_name = format!(
        "{}.expanded.cairo",
        compilation_unit
            .main_component()
            .first_target()
            .name
            .clone()
    );
    let target_dir = compilation_unit.target_dir(ws);
    write_string(file_name.as_str(), "output file", &target_dir, ws, content)?;
    Ok(())
}

fn format_cairo(content: String) -> Option<String> {
    let formatter = CairoFormatter::new(FormatterConfig::default());
    let content = formatter.format_to_string(&content).ok()?;
    // Get formatted string, whether any changes have been made, or not.
    Some(match content {
        FormatOutcome::Identical(value) => value,
        FormatOutcome::DiffFound(diff) => diff.formatted,
    })
}

fn suppress_error(err: &anyhow::Error) -> bool {
    matches!(err.downcast_ref(), Some(&DiagnosticsError))
}
