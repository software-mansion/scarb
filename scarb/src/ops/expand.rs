use crate::compiler::db::{build_scarb_root_database, ScarbDatabase};
use crate::compiler::helpers::{build_compiler_config, write_string};
use crate::compiler::{CairoCompilationUnit, CompilationUnit, CompilationUnitAttributes};
use crate::core::{Package, TargetKind, Workspace};
use crate::ops;
use crate::ops::{get_test_package_ids, FeaturesOpts};
use anyhow::{bail, Context, Result};
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
use smol_str::SmolStr;
use std::collections::HashSet;

#[derive(Clone, Debug)]
pub struct ExpandOpts {
    pub features: FeaturesOpts,
    pub target_kind: Option<TargetKind>,
    pub target_name: Option<SmolStr>,
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

    let compilation_units = compilation_units
        .into_iter()
        // We rewrite group compilation units to single source paths ones. We value simplicity over
        // performance here, as expand output will be read by people rather than tooling.
        .flat_map(|unit| match unit {
            CompilationUnit::Cairo(unit) => unit
                .rewrite_to_single_source_paths()
                .into_iter()
                .map(CompilationUnit::Cairo)
                .collect::<Vec<_>>(),
            // We include non-cairo compilation units here, so we can show better error msg later.
            _ => vec![unit],
        })
        .filter(|unit| {
            let target_kind = if opts.target_name.is_none() && opts.target_kind.is_none() {
                // If no target specifier is used - default to lib.
                Some(TargetKind::LIB)
            } else {
                opts.target_kind.clone()
            };
            // Includes test package ids.
            get_test_package_ids(vec![package.id], ws).contains(&unit.main_package_id())
                // We can use main_component below, as targets are not grouped.
                && target_kind.as_ref()
                    .map_or(true, |kind| unit.main_component().target_kind() == *kind)
                && opts
                    .target_name
                    .as_ref()
                    .map_or(true, |name| unit.main_component().first_target().name == *name)
        })
        .map(|unit| match unit {
            CompilationUnit::Cairo(unit) => Ok(unit),
            _ => bail!("only cairo compilation units can be expanded"),
        })
        .collect::<Result<Vec<_>>>()?;

    if compilation_units.is_empty() {
        bail!("no compilation units found for `{package_name}`")
    }

    for compilation_unit in compilation_units {
        do_expand(&compilation_unit, opts.clone(), ws)?;
    }

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
    compilation_unit: &CairoCompilationUnit,
    opts: ExpandOpts,
    ws: &Workspace<'_>,
) -> Result<()> {
    let ScarbDatabase { db, .. } = build_scarb_root_database(compilation_unit, ws)?;
    let mut compiler_config = build_compiler_config(compilation_unit, ws);
    // Report diagnostics, but do not fail.
    let _ = compiler_config.diagnostics_reporter.check(&db);
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
    let item_asts = file_syntax.items(&db);

    let mut builder = PatchBuilder::new(&db, &item_asts);
    let mut module_stack = ModuleStack::new();

    for module_id in crate_modules.iter() {
        builder.add_str(module_stack.register(module_id.full_path(&db)).as_str());
        let Some(module_items) = db.module_items(*module_id).to_option() else {
            continue;
        };
        let mut seen_uses = HashSet::new();
        for item_id in module_items.iter() {
            // We need to handle uses manually, as module data only includes use leaf instead of path.
            if let ModuleItemId::Use(use_id) = item_id {
                let use_item = use_id.stable_ptr(&db).lookup(db.upcast());
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
            let node = item_id.stable_location(&db).syntax_node(&db);
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
