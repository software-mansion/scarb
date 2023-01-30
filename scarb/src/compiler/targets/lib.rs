use std::io::Write;
use std::mem;

use anyhow::{Context, Result};
use cairo_lang_compiler::db::RootDatabase;
use cairo_lang_compiler::project::{ProjectConfig, ProjectConfigContent};
use cairo_lang_compiler::CompilerConfig;
use cairo_lang_filesystem::db::FilesGroup;
use cairo_lang_filesystem::ids::{CrateLongId, Directory};
use cairo_lang_sierra_to_casm::metadata::{calc_metadata, MetadataComputationConfig};
use tracing::{span, trace, Level};

use crate::compiler::CompilationUnit;
use crate::core::{Config, LibTargetKind, PackageName, Workspace};
use crate::ui::TypedMessage;

#[tracing::instrument(level = "trace", skip_all, fields(unit = unit.name()))]
pub fn compile_lib(unit: CompilationUnit, ws: &Workspace<'_>) -> Result<()> {
    let props = unit.target.kind.downcast::<LibTargetKind>();
    if !props.sierra && !props.casm {
        ws.config().ui().warn(
            "both Sierra and CASM lib targets have been disabled, \
            Scarb will not produce anything",
        );
    }

    let target_dir = unit.profile.target_dir(ws.config());

    let db = {
        let project_config = build_project_config(&unit)?;
        trace!(project_config = ?project_config);

        let mut b = RootDatabase::builder();
        b.with_project_config(project_config);
        b.build()
    };

    let compiler_config = CompilerConfig {
        on_diagnostic: {
            // UNSAFE: We are not actually creating a dangling `Config` reference here,
            //   because diagnostic callback by definition should rather be dropped
            //   when compilation ends.
            let config: &'static Config = unsafe { mem::transmute(ws.config()) };
            Some(Box::new({
                |diagnostic: String| {
                    config
                        .ui()
                        .print(TypedMessage::naked_text("diagnostic", &diagnostic));
                }
            }))
        },
        ..CompilerConfig::default()
    };

    let main_crate_ids = vec![db.intern_crate(CrateLongId(unit.package.id.name.to_smol_str()))];

    let sierra_program = {
        let _ = span!(Level::TRACE, "compile_sierra").enter();
        cairo_lang_compiler::compile_prepared_db(db, main_crate_ids, compiler_config)?
    };

    if props.sierra {
        let mut file = target_dir.open_rw(
            format!("{}.sierra", unit.target.name),
            "output file",
            ws.config(),
        )?;
        file.write_all(sierra_program.to_string().as_bytes())?;
    }

    if props.casm {
        let gas_usage_check = true;

        let metadata = {
            let _ = span!(Level::TRACE, "casm_calc_metadata");
            calc_metadata(&sierra_program, MetadataComputationConfig::default())
                .context("failed calculating Sierra variables")?
        };

        let cairo_program = {
            let _ = span!(Level::TRACE, "compile_casm");
            cairo_lang_sierra_to_casm::compiler::compile(
                &sierra_program,
                &metadata,
                gas_usage_check,
            )?
        };

        let mut file = target_dir.open_rw(
            format!("{}.casm", unit.target.name),
            "output file",
            ws.config(),
        )?;
        file.write_all(cairo_program.to_string().as_bytes())?;
    }

    Ok(())
}

fn build_project_config(unit: &CompilationUnit) -> Result<ProjectConfig> {
    let crate_roots = unit
        .components
        .iter()
        .filter(|pkg| pkg.id.name != PackageName::CORE)
        .map(|pkg| {
            (
                pkg.id.name.to_smol_str(),
                pkg.source_dir().into_std_path_buf(),
            )
        })
        .collect();

    let corelib = unit
        .components
        .iter()
        .find(|pkg| pkg.id.name == PackageName::CORE)
        .map(|pkg| Directory(pkg.source_dir().into_std_path_buf()));

    let content = ProjectConfigContent { crate_roots };

    Ok(ProjectConfig {
        base_path: unit.package.root().into(),
        corelib,
        content,
    })
}
