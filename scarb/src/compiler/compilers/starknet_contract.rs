use anyhow::{bail, ensure, Context, Result};
use cairo_lang_compiler::db::RootDatabase;
use cairo_lang_compiler::CompilerConfig;
use cairo_lang_defs::ids::NamedLanguageElementId;
use cairo_lang_filesystem::db::{AsFilesGroupMut, FilesGroup};
use cairo_lang_filesystem::flag::Flag;
use cairo_lang_filesystem::ids::{CrateId, CrateLongId, FlagId};
use cairo_lang_semantic::db::SemanticGroup;
use cairo_lang_starknet::compile::compile_prepared_db;
use cairo_lang_starknet::contract::{find_contracts, ContractDeclaration};
use cairo_lang_starknet_classes::allowed_libfuncs::{
    AllowedLibfuncsError, ListSelector, BUILTIN_EXPERIMENTAL_LIBFUNCS_LIST,
};
use cairo_lang_starknet_classes::casm_contract_class::CasmContractClass;
use cairo_lang_starknet_classes::contract_class::ContractClass;
use cairo_lang_utils::{Upcast, UpcastMut};
use indoc::{formatdoc, writedoc};
use itertools::{izip, Itertools};
use serde::{Deserialize, Serialize};
use smol_str::SmolStr;
use std::collections::HashSet;
use std::fmt::Write;
use std::iter::zip;
use tracing::{debug, trace, trace_span};

use crate::compiler::helpers::{build_compiler_config, collect_main_crate_ids, write_json};
use crate::compiler::{CairoCompilationUnit, CompilationUnitAttributes, Compiler};
use crate::core::{PackageName, TargetKind, Utf8PathWorkspaceExt, Workspace};
use crate::flock::Filesystem;
use crate::internal::serdex::RelativeUtf8PathBuf;
use scarb_stable_hash::short_hash;
use scarb_ui::Ui;

const CAIRO_PATH_SEPARATOR: &str = "::";
const GLOB_PATH_SELECTOR: &str = "*";

// TODO(#111): starknet-contract should be implemented as an extension.
pub struct StarknetContractCompiler;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Props {
    pub sierra: bool,
    pub casm: bool,
    pub casm_add_pythonic_hints: bool,
    pub allowed_libfuncs: bool,
    pub allowed_libfuncs_deny: bool,
    pub allowed_libfuncs_list: Option<SerdeListSelector>,
    pub build_external_contracts: Option<Vec<ContractSelector>>,
}

impl Default for Props {
    fn default() -> Self {
        Self {
            sierra: true,
            casm: false,
            casm_add_pythonic_hints: false,
            allowed_libfuncs: true,
            allowed_libfuncs_deny: false,
            allowed_libfuncs_list: None,
            build_external_contracts: None,
        }
    }
}

// FIXME(#401): Make allowed-libfuncs-list.path relative to current Scarb.toml rather than PWD.
#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged, rename_all = "kebab-case")]
pub enum SerdeListSelector {
    Name { name: String },
    Path { path: RelativeUtf8PathBuf },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContractSelector(pub String);

impl ContractSelector {
    fn package(&self) -> PackageName {
        let parts = self
            .0
            .split_once(CAIRO_PATH_SEPARATOR)
            .unwrap_or((self.0.as_str(), ""));
        PackageName::new(parts.0)
    }

    fn contract(&self) -> String {
        let parts = self
            .0
            .rsplit_once(CAIRO_PATH_SEPARATOR)
            .unwrap_or((self.0.as_str(), ""));
        parts.1.to_string()
    }

    fn is_wildcard(&self) -> bool {
        self.0.ends_with(GLOB_PATH_SELECTOR)
    }

    fn partial_path(&self) -> String {
        let parts = self
            .0
            .split_once(GLOB_PATH_SELECTOR)
            .unwrap_or((self.0.as_str(), ""));
        parts.0.to_string()
    }

    fn full_path(&self) -> String {
        self.0.clone()
    }
}

struct ContractFileStemCalculator(HashSet<String>);

impl ContractFileStemCalculator {
    fn new(contract_paths: Vec<String>) -> Self {
        let mut seen = HashSet::new();
        let contract_name_duplicates = contract_paths
            .iter()
            .map(|it| ContractSelector(it.clone()).contract())
            .filter(|contract_name| {
                // insert returns false for duplicate values
                !seen.insert(contract_name.clone())
            })
            .collect::<HashSet<String>>();
        Self(contract_name_duplicates)
    }

    fn get_stem(&mut self, full_path: String) -> String {
        let contract_selector = ContractSelector(full_path);
        let contract_name = contract_selector.contract();

        if self.0.contains(&contract_name) {
            contract_selector
                .full_path()
                .replace(CAIRO_PATH_SEPARATOR, "_")
        } else {
            contract_name
        }
    }
}

#[derive(Debug, Serialize)]
struct StarknetArtifacts {
    version: usize,
    contracts: Vec<ContractArtifacts>,
}

impl Default for StarknetArtifacts {
    fn default() -> Self {
        Self {
            version: 1,
            contracts: Vec::new(),
        }
    }
}

impl StarknetArtifacts {
    fn finish(&mut self) {
        assert!(
            self.contracts.iter().map(|it| &it.id).all_unique(),
            "Artifacts IDs must be unique."
        );

        self.contracts.sort_unstable_by_key(|it| it.id.clone());
    }
}

#[derive(Debug, Serialize)]
struct ContractArtifacts {
    id: String,
    package_name: PackageName,
    contract_name: String,
    module_path: String,
    artifacts: ContractArtifact,
}

impl ContractArtifacts {
    fn new(
        package_name: &PackageName,
        contract_name: &str,
        contract_path: &str,
        module_path: &str,
    ) -> Self {
        Self {
            id: short_hash((&package_name, &contract_path)),
            package_name: package_name.clone(),
            contract_name: contract_name.to_owned(),
            module_path: module_path.to_owned(),
            artifacts: ContractArtifact::default(),
        }
    }
}

#[derive(Debug, Default, Serialize)]
struct ContractArtifact {
    sierra: Option<String>,
    casm: Option<String>,
}

const AUTO_WITHDRAW_GAS_FLAG: &str = "add_withdraw_gas";

impl Compiler for StarknetContractCompiler {
    fn target_kind(&self) -> TargetKind {
        TargetKind::STARKNET_CONTRACT.clone()
    }

    fn compile(
        &self,
        unit: CairoCompilationUnit,
        db: &mut RootDatabase,
        ws: &Workspace<'_>,
    ) -> Result<()> {
        let props: Props = unit.main_component().target_props()?;
        if !props.sierra && !props.casm {
            ws.config().ui().warn(
                "both Sierra and CASM Starknet contract targets have been disabled, \
                Scarb will not produce anything",
            );
        }

        ensure_gas_enabled(db)?;

        if let Some(external_contracts) = props.build_external_contracts.clone() {
            for path in external_contracts.iter() {
                ensure!(path.0.matches(GLOB_PATH_SELECTOR).count() <= 1,
                    "external contract path `{}` has multiple global path selectors, only one '*' selector is allowed",
                    path.0);
            }
        }

        let target_dir = unit.target_dir(ws);

        let main_crate_ids = collect_main_crate_ids(&unit, db);

        let compiler_config = build_compiler_config(db, &unit, &main_crate_ids, ws);

        let Compiled {
            contract_paths,
            contracts,
            classes,
        } = get_compiled_contracts(
            main_crate_ids,
            props.build_external_contracts.clone(),
            compiler_config,
            db,
            ws,
        )?;

        check_allowed_libfuncs(&props, &contracts, &classes, db, &unit, ws)?;

        let casm_classes: Vec<Option<CasmContractClass>> = if props.casm {
            let _ = trace_span!("compile_sierra").enter();
            zip(&contracts, &classes)
                .map(|(decl, class)| -> Result<_> {
                    let contract_name = decl.submodule_id.name(db.upcast_mut());
                    let casm_class = CasmContractClass::from_contract_class(
                        class.clone(),
                        props.casm_add_pythonic_hints,
                        usize::MAX,
                    )
                    .with_context(|| {
                        format!("{contract_name}: failed to compile Sierra contract to CASM")
                    })?;
                    Ok(Some(casm_class))
                })
                .try_collect()?
        } else {
            classes.iter().map(|_| None).collect()
        };

        let target_name = &unit.main_component().target_name();

        let writer = ArtifactsWriter::new(target_name.clone(), target_dir, props);
        writer.write(contract_paths, &contracts, &classes, &casm_classes, db, ws)?;

        Ok(())
    }
}

pub struct ArtifactsWriter {
    sierra: bool,
    casm: bool,
    target_dir: Filesystem,
    target_name: SmolStr,
}

impl ArtifactsWriter {
    pub fn new(target_name: SmolStr, target_dir: Filesystem, props: Props) -> Self {
        Self {
            sierra: props.sierra,
            casm: props.casm,
            target_dir,
            target_name,
        }
    }

    pub fn write(
        self,
        contract_paths: Vec<String>,
        contracts: &Vec<ContractDeclaration>,
        classes: &[ContractClass],
        casm_classes: &[Option<CasmContractClass>],
        db: &mut RootDatabase,
        ws: &Workspace<'_>,
    ) -> Result<()> {
        let mut artifacts = StarknetArtifacts::default();
        let mut file_stem_calculator = ContractFileStemCalculator::new(contract_paths);

        for (decl, class, casm_class) in izip!(contracts, classes, casm_classes) {
            let contract_name = decl.submodule_id.name(db.upcast_mut());
            let contract_path = decl.module_id().full_path(db.upcast_mut());

            let contract_selector = ContractSelector(contract_path);
            let package_name = contract_selector.package();
            let contract_stem = file_stem_calculator.get_stem(contract_selector.full_path());

            let file_stem = format!("{}_{contract_stem}", self.target_name);

            let mut artifact = ContractArtifacts::new(
                &package_name,
                &contract_name,
                contract_selector.full_path().as_str(),
                &decl.module_id().full_path(db.upcast_mut()),
            );

            if self.sierra {
                let file_name = format!("{file_stem}.contract_class.json");
                write_json(&file_name, "output file", &self.target_dir, ws, class)?;
                artifact.artifacts.sierra = Some(file_name);
            }

            if self.casm {
                if let Some(casm_class) = casm_class {
                    let file_name = format!("{file_stem}.compiled_contract_class.json");
                    write_json(&file_name, "output file", &self.target_dir, ws, casm_class)?;
                    artifact.artifacts.casm = Some(file_name);
                }
            }

            artifacts.contracts.push(artifact);
        }

        artifacts.finish();

        write_json(
            &format!("{}.starknet_artifacts.json", self.target_name),
            "starknet artifacts file",
            &self.target_dir,
            ws,
            &artifacts,
        )?;

        Ok(())
    }
}

pub struct Compiled {
    pub contract_paths: Vec<String>,
    pub contracts: Vec<ContractDeclaration>,
    pub classes: Vec<ContractClass>,
}

pub fn get_compiled_contracts(
    main_crate_ids: Vec<CrateId>,
    build_external_contracts: Option<Vec<ContractSelector>>,
    compiler_config: CompilerConfig<'_>,
    db: &mut RootDatabase,
    ws: &Workspace<'_>,
) -> Result<Compiled> {
    let contracts = find_project_contracts(
        db.upcast_mut(),
        ws.config().ui(),
        main_crate_ids,
        build_external_contracts,
    )?;

    let contract_paths = contracts
        .iter()
        .map(|decl| decl.module_id().full_path(db.upcast_mut()))
        .collect::<Vec<_>>();
    trace!(contracts = ?contract_paths);

    let classes = {
        let _ = trace_span!("compile_starknet").enter();
        compile_prepared_db(db, &contracts.iter().collect::<Vec<_>>(), compiler_config)?
    };
    Ok(Compiled {
        contract_paths,
        contracts,
        classes,
    })
}

pub fn ensure_gas_enabled(db: &mut RootDatabase) -> Result<()> {
    let flag = FlagId::new(db.as_files_group_mut(), AUTO_WITHDRAW_GAS_FLAG);
    let flag = db.get_flag(flag);
    ensure!(
        flag.map(|f| matches!(*f, Flag::AddWithdrawGas(true)))
            .unwrap_or(false),
        "the target starknet contract compilation requires gas to be enabled"
    );
    Ok(())
}

fn find_project_contracts(
    mut db: &dyn SemanticGroup,
    ui: Ui,
    main_crate_ids: Vec<CrateId>,
    external_contracts: Option<Vec<ContractSelector>>,
) -> Result<Vec<ContractDeclaration>> {
    let internal_contracts = {
        let _ = trace_span!("find_internal_contracts").enter();
        find_contracts(db, &main_crate_ids)
    };

    let external_contracts: Vec<ContractDeclaration> =
        if let Some(external_contracts) = external_contracts {
            let _ = trace_span!("find_external_contracts").enter();
            debug!("external contracts selectors: {:?}", external_contracts);

            let crate_ids = external_contracts
                .iter()
                .map(|selector| selector.package().into())
                .unique()
                .map(|package_name: SmolStr| {
                    db.upcast_mut()
                        .intern_crate(CrateLongId::Real(package_name))
                })
                .collect::<Vec<_>>();
            let contracts = find_contracts(db, crate_ids.as_ref());
            let filtered_contracts: Vec<ContractDeclaration> = contracts
                .into_iter()
                .filter(|decl| {
                    let contract_path = decl.module_id().full_path(db.upcast());
                    external_contracts
                        .iter()
                        .any(|selector| contract_matches(selector, contract_path.as_str()))
                })
                .collect();

            let never_matched = external_contracts
                .iter()
                .filter(|selector| {
                    !filtered_contracts.iter().any(|decl| {
                        let contract_path = decl.module_id().full_path(db.upcast());
                        contract_matches(selector, contract_path.as_str())
                    })
                })
                .collect_vec();
            if !never_matched.is_empty() {
                let never_matched = never_matched
                    .iter()
                    .map(|selector| selector.full_path())
                    .collect_vec()
                    .join("`, `");
                ui.warn(format!(
                    "external contracts not found for selectors: `{never_matched}`"
                ));
            }

            filtered_contracts
        } else {
            debug!("no external contracts selected");
            Vec::new()
        };

    Ok(internal_contracts
        .into_iter()
        .chain(external_contracts)
        .collect())
}

fn contract_matches(selector: &ContractSelector, contract_path: &str) -> bool {
    if selector.is_wildcard() {
        contract_path.starts_with(&selector.partial_path())
    } else {
        contract_path == selector.full_path()
    }
}

fn check_allowed_libfuncs(
    props: &Props,
    contracts: &[ContractDeclaration],
    classes: &[ContractClass],
    db: &RootDatabase,
    unit: &CairoCompilationUnit,
    ws: &Workspace<'_>,
) -> Result<()> {
    if !props.allowed_libfuncs {
        debug!("allowed libfuncs checking disabled by target props");
        return Ok(());
    }

    let list_selector = match &props.allowed_libfuncs_list {
        Some(SerdeListSelector::Name { name }) => ListSelector::ListName(name.clone()),
        Some(SerdeListSelector::Path { path }) => {
            let path = path.relative_to_file(unit.main_component().package.manifest_path())?;
            ListSelector::ListFile(path.into_string())
        }
        None => Default::default(),
    };

    let mut found_disallowed = false;
    for (decl, class) in zip(contracts, classes) {
        match class.validate_version_compatible(list_selector.clone()) {
            Ok(()) => {}

            Err(AllowedLibfuncsError::UnsupportedLibfunc {
                invalid_libfunc,
                allowed_libfuncs_list_name,
            }) => {
                found_disallowed = true;

                let contract_name = decl.submodule_id.name(db.upcast());
                let mut diagnostic = formatdoc! {r#"
                    libfunc `{invalid_libfunc}` is not allowed in the libfuncs list `{allowed_libfuncs_list_name}`
                     --> contract: {contract_name}
                "#};

                // If user did not explicitly specify the allowlist, show a help message
                // instructing how to do this. Otherwise, we know that user knows what they
                // do, so we do not clutter compiler output.
                if list_selector == Default::default() {
                    let experimental = BUILTIN_EXPERIMENTAL_LIBFUNCS_LIST;

                    let scarb_toml = unit
                        .main_component()
                        .package
                        .manifest_path()
                        .workspace_relative(ws);

                    let _ = writedoc!(
                        &mut diagnostic,
                        r#"
                            help: try compiling with the `{experimental}` list
                             --> {scarb_toml}
                                [[target.starknet-contract]]
                                allowed-libfuncs-list.name = "{experimental}"
                        "#
                    );
                }

                if props.allowed_libfuncs_deny {
                    ws.config().ui().error(diagnostic);
                } else {
                    ws.config().ui().warn(diagnostic);
                }
            }

            Err(e) => {
                return Err(e).with_context(|| {
                    format!(
                        "failed to check allowed libfuncs for contract: {contract_name}",
                        contract_name = decl.submodule_id.name(db.upcast())
                    )
                })
            }
        }
    }

    if found_disallowed && props.allowed_libfuncs_deny {
        bail!("aborting compilation, because contracts use disallowed Sierra libfuncs");
    }

    Ok(())
}
