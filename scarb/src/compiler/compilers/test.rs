use anyhow::Result;
use cairo_lang_compiler::db::RootDatabase;
use cairo_lang_filesystem::cfg::CfgSet;
use cairo_lang_filesystem::db::{CrateConfiguration, CrateSettings, FilesGroup, FilesGroupEx};
use cairo_lang_test_plugin::compile_test_prepared_db;
use tracing::trace_span;

use crate::compiler::helpers::{
    build_compiler_config, collect_all_crate_ids, collect_main_crate_ids,
    collect_non_test_crate_ids, write_json,
};
use crate::compiler::{CompilationUnit, Compiler};
use crate::core::{PackageName, SourceId, TargetKind, Workspace};

pub struct TestCompiler;

impl Compiler for TestCompiler {
    fn target_kind(&self) -> TargetKind {
        TargetKind::TEST.clone()
    }

    fn compile(
        &self,
        unit: CompilationUnit,
        db: &mut RootDatabase,
        ws: &Workspace<'_>,
    ) -> Result<()> {
        let target_dir = unit.target_dir(ws);

        let test_crate_ids = collect_main_crate_ids(&unit, db);
        let main_crate_ids = collect_all_crate_ids(&unit, db);
        let starknet = unit.cairo_plugins.iter().any(|plugin| {
            plugin.package.id.name == PackageName::STARKNET
                && plugin.package.id.source_id == SourceId::for_std()
        });

        let non_test_crate_ids = collect_non_test_crate_ids(&unit, db);

        for crate_id in non_test_crate_ids {
            if let Some(cfg) = db.crate_config(crate_id) {
                db.set_crate_config(
                    crate_id,
                    Some(CrateConfiguration {
                        root: cfg.root,
                        settings: CrateSettings {
                            cfg_set: Some(CfgSet::from_iter(
                                db.cfg_set().iter().filter(|cfg| cfg.key != "test").cloned(),
                            )),
                            ..cfg.settings
                        },
                    }),
                )
            }
        }

        let diagnostics_reporter = build_compiler_config(&unit, ws).diagnostics_reporter;

        diagnostics_reporter
            .with_crates(&main_crate_ids)
            .ensure(db)?;

        let test_compilation = {
            let _ = trace_span!("compile_test").enter();
            compile_test_prepared_db(db, starknet, main_crate_ids, test_crate_ids)?
        };

        {
            let _ = trace_span!("serialize_test").enter();
            let file_name = format!("{}.test.json", unit.target().name);
            write_json(
                &file_name,
                "output file",
                &target_dir,
                ws,
                &test_compilation,
            )?;
        }

        Ok(())
    }
}
