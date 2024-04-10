use anyhow::{anyhow, bail, Context, Result};
use cairo_lang_compiler::db::RootDatabase;
use cairo_lang_compiler::diagnostics::DiagnosticsReporter;
use cairo_lang_debug::DebugWithDb;
use cairo_lang_defs::db::DefsGroup;
use cairo_lang_defs::ids::{FreeFunctionId, FunctionWithBodyId, ModuleId, ModuleItemId};
use cairo_lang_diagnostics::ToOption;
use cairo_lang_filesystem::cfg::{Cfg, CfgSet};
use cairo_lang_filesystem::db::{
    AsFilesGroupMut, CrateConfiguration, CrateSettings, FilesGroup, FilesGroupEx,
};
use cairo_lang_filesystem::ids::{CrateId, CrateLongId, Directory};
use cairo_lang_lowering::ids::ConcreteFunctionWithBodyId;
use cairo_lang_project::{ProjectConfig, ProjectConfigContent};
use cairo_lang_semantic::db::SemanticGroup;
use cairo_lang_semantic::items::functions::GenericFunctionId;
use cairo_lang_semantic::{ConcreteFunction, FunctionLongId};
use cairo_lang_sierra::debug_info::{Annotations, DebugInfo};
use cairo_lang_sierra::extensions::enm::EnumType;
use cairo_lang_sierra::extensions::NamedType;
use cairo_lang_sierra::ids::GenericTypeId;
use cairo_lang_sierra::program::{GenericArg, ProgramArtifact};
use cairo_lang_sierra_generator::db::SierraGenGroup;
use cairo_lang_sierra_generator::replace_ids::replace_sierra_ids_in_program;
use cairo_lang_starknet::starknet_plugin_suite;
use cairo_lang_test_plugin::test_plugin_suite;
use cairo_lang_utils::ordered_hash_map::OrderedHashMap;
use itertools::Itertools;
use serde::Serialize;
use smol_str::SmolStr;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::compilation::test_collector::config::{ExpectedTestResult, RawForkConfig};
use crate::metadata::CompilationUnit;
use config::{forge_try_extract_test_config, FuzzerConfig, SingleTestConfig};
use function_finder::FunctionFinder;
use plugin::snforge_test_plugin_suite;

mod config;
mod function_finder;
mod plugin;

fn find_all_tests(
    db: &dyn SemanticGroup,
    main_crate: CrateId,
) -> Vec<(FreeFunctionId, SingleTestConfig)> {
    let mut tests = vec![];
    let modules = db.crate_modules(main_crate);
    for module_id in modules.iter() {
        let Ok(module_items) = db.module_items(*module_id) else {
            continue;
        };
        tests.extend(module_items.iter().filter_map(|item| {
            let ModuleItemId::FreeFunction(func_id) = item else {
                return None;
            };
            let Ok(attrs) = db.function_with_body_attributes(FunctionWithBodyId::Free(*func_id))
            else {
                return None;
            };
            Some((
                *func_id,
                forge_try_extract_test_config(db.upcast(), &attrs).unwrap()?,
            ))
        }));
    }
    tests
}

#[derive(Debug, PartialEq, Clone, Serialize)]
pub struct TestCaseRaw {
    pub name: String,
    pub available_gas: Option<usize>,
    pub ignored: bool,
    pub expected_result: ExpectedTestResult,
    pub fork_config: Option<RawForkConfig>,
    pub fuzzer_config: Option<FuzzerConfig>,
    pub test_details: TestDetails,
}

#[derive(Debug, PartialEq, Clone, Serialize)]
pub struct TestDetails {
    pub entry_point_offset: usize,
    pub parameter_types: Vec<(GenericTypeId, i16)>,
    pub return_types: Vec<(GenericTypeId, i16)>,
}

pub fn collect_tests(
    crate_name: &str,
    crate_root: &Path,
    lib_content: &str,
    compilation_unit: &CompilationUnit,
) -> Result<(ProgramArtifact, Vec<TestCaseRaw>)> {
    let crate_roots: OrderedHashMap<SmolStr, PathBuf> = compilation_unit
        .dependencies()
        .iter()
        .cloned()
        .map(|source_root| (source_root.name.into(), source_root.path))
        .collect();

    let project_config = ProjectConfig {
        base_path: crate_root.into(),
        corelib: Some(Directory::Real(compilation_unit.corelib_path()?)),
        content: ProjectConfigContent {
            crate_roots,
            crates_config: compilation_unit.crates_config_for_compilation_unit(),
        },
    };

    // code taken from crates/cairo-lang-test-runner/src/lib.rs
    let db = &mut {
        let mut b = RootDatabase::builder();
        b.with_cfg(CfgSet::from_iter([Cfg::name("test")]));
        b.with_plugin_suite(snforge_test_plugin_suite());
        b.with_plugin_suite(test_plugin_suite());
        b.with_plugin_suite(starknet_plugin_suite());
        b.with_project_config(project_config);
        b.build()?
    };

    let main_package_crate_settings = compilation_unit.main_package_crate_settings();
    let main_crate_id = insert_lib_entrypoint_content_into_db(
        db,
        crate_name,
        crate_root,
        lib_content,
        main_package_crate_settings,
    );

    if build_diagnostics_reporter(compilation_unit).check(db) {
        return Err(anyhow!(
            "Failed to compile test artifact, for detailed information go through the logs above"
        ));
    }
    let all_tests = find_all_tests(db, main_crate_id);

    let z: Vec<ConcreteFunctionWithBodyId> = all_tests
        .iter()
        .filter_map(|(func_id, _cfg)| {
            ConcreteFunctionWithBodyId::from_no_generics_free(db, *func_id)
        })
        .collect();

    let sierra_program = db
        .get_sierra_program_for_functions(z)
        .to_option()
        .context("Compilation failed without any diagnostics")
        .context("Failed to get sierra program")?;

    let debug_annotations = if compilation_unit.unstable_add_statements_functions_debug_info() {
        Some(Annotations::from(
            sierra_program
                .debug_info
                .statements_locations
                .extract_statements_functions(db),
        ))
    } else {
        None
    };
    let debug_info = debug_annotations.map(|annotations| DebugInfo {
        type_names: Default::default(),
        libfunc_names: Default::default(),
        user_func_names: Default::default(),
        annotations,
    });

    let sierra_program = replace_sierra_ids_in_program(db, &sierra_program.program);
    let function_finder = FunctionFinder::new(sierra_program.clone())?;

    let collected_tests = all_tests
        .into_iter()
        .map(|(func_id, test)| {
            (
                format!(
                    "{:?}",
                    FunctionLongId {
                        function: ConcreteFunction {
                            generic_function: GenericFunctionId::Free(func_id),
                            generic_args: vec![]
                        }
                    }
                    .debug(db)
                ),
                test,
            )
        })
        .collect_vec()
        .into_iter()
        .map(|(test_name, config)| {
            let test_details = build_test_details(&function_finder, &test_name).unwrap();
            TestCaseRaw {
                name: test_name,
                available_gas: config.available_gas,
                ignored: config.ignored,
                expected_result: config.expected_result,
                fork_config: config.fork_config,
                fuzzer_config: config.fuzzer_config,
                test_details,
            }
        })
        .collect();

    validate_tests(&function_finder, &collected_tests)?;

    Ok((
        ProgramArtifact {
            program: sierra_program,
            debug_info,
        },
        collected_tests,
    ))
}

fn build_test_details(function_finder: &FunctionFinder, test_name: &str) -> Result<TestDetails> {
    let func = function_finder.find_function(test_name)?;

    let parameter_types =
        function_finder.generic_id_and_size_from_concrete(&func.signature.param_types);
    let return_types = function_finder.generic_id_and_size_from_concrete(&func.signature.ret_types);

    Ok(TestDetails {
        entry_point_offset: func.entry_point.0,
        parameter_types,
        return_types,
    })
}

fn build_diagnostics_reporter(compilation_unit: &CompilationUnit) -> DiagnosticsReporter<'static> {
    if compilation_unit.allow_warnings() {
        DiagnosticsReporter::stderr().allow_warnings()
    } else {
        DiagnosticsReporter::stderr()
    }
}

// inspired with cairo-lang-compiler/src/project.rs:49 (part of setup_single_project_file)
fn insert_lib_entrypoint_content_into_db(
    db: &mut RootDatabase,
    crate_name: &str,
    crate_root: &Path,
    lib_content: &str,
    main_package_crate_settings: CrateSettings,
) -> CrateId {
    let main_crate_id = db.intern_crate(CrateLongId::Real(SmolStr::from(crate_name)));
    db.set_crate_config(
        main_crate_id,
        Some(CrateConfiguration {
            root: Directory::Real(crate_root.to_path_buf()),
            settings: main_package_crate_settings,
        }),
    );

    let module_id = ModuleId::CrateRoot(main_crate_id);
    let file_id = db.module_main_file(module_id).unwrap();
    db.as_files_group_mut()
        .override_file_content(file_id, Some(Arc::new(lib_content.to_string())));

    main_crate_id
}

fn validate_tests(
    function_finder: &FunctionFinder,
    collected_tests: &Vec<TestCaseRaw>,
) -> Result<(), anyhow::Error> {
    for test in collected_tests {
        let func = function_finder.find_function(&test.name)?;
        let signature = &func.signature;
        let ret_types = &signature.ret_types;
        if ret_types.is_empty() {
            bail!(
                "The test function {} always succeeds and cannot be used as a test. Make sure to include panickable statements such as `assert` in your test",
                test.name
            );
        }
        let tp = &ret_types[ret_types.len() - 1];
        let info = function_finder.get_info(tp);
        let mut maybe_return_type_name = None;
        if info.long_id.generic_id == EnumType::ID {
            if let GenericArg::UserType(ut) = &info.long_id.generic_args[0] {
                if let Some(name) = ut.debug_name.as_ref() {
                    maybe_return_type_name = Some(name.as_str());
                }
            }
        }
        if let Some(return_type_name) = maybe_return_type_name {
            if !return_type_name.starts_with("core::panics::PanicResult::") {
                bail!(
                    "The test function {} always succeeds and cannot be used as a test. Make sure to include panickable statements such as `assert` in your test",
                    test.name
                );
            }
            if return_type_name != "core::panics::PanicResult::<((),)>" {
                bail!(
                    "Test function {} returns a value {}, it is required that test functions do \
                     not return values",
                    test.name,
                    return_type_name
                );
            }
        } else {
            bail!(
                "Couldn't read result type for test function {} possible cause: The test function {} \
                 always succeeds and cannot be used as a test. Make sure to include panickable statements such as `assert` in your test",
                test.name,
                test.name
            );
        }
    }

    Ok(())
}
