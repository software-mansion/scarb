use crate::compiler::compilers::starknet_contract::{ContractFileStemCalculator, ContractSelector};
use crate::compiler::compilers::Props;
use crate::compiler::helpers::write_json;
use crate::core::{PackageName, Workspace};
use crate::flock::Filesystem;
use cairo_lang_compiler::db::RootDatabase;
use cairo_lang_defs::ids::NamedLanguageElementId;
use cairo_lang_starknet::contract::ContractDeclaration;
use cairo_lang_starknet_classes::casm_contract_class::CasmContractClass;
use cairo_lang_starknet_classes::contract_class::ContractClass;
use cairo_lang_utils::UpcastMut;
use itertools::{izip, Itertools};
use scarb_stable_hash::short_hash;
use serde::Serialize;
use smol_str::SmolStr;

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
        package_name: PackageName,
        contract_name: &str,
        contract_path: &str,
        module_path: &str,
    ) -> Self {
        Self {
            id: short_hash((&package_name, &contract_path)),
            package_name,
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

pub struct ArtifactsWriter {
    sierra: bool,
    casm: bool,
    target_dir: Filesystem,
    target_name: SmolStr,
    extension_prefix: Option<String>,
}

impl ArtifactsWriter {
    pub fn new(target_name: SmolStr, target_dir: Filesystem, props: Props) -> Self {
        Self {
            sierra: props.sierra,
            casm: props.casm,
            target_dir,
            target_name,
            extension_prefix: None,
        }
    }

    pub fn with_extension_prefix(self, prefix: String) -> Self {
        Self {
            extension_prefix: Some(prefix),
            ..self
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
    ) -> anyhow::Result<()> {
        let mut artifacts = StarknetArtifacts::default();
        let mut file_stem_calculator = ContractFileStemCalculator::new(contract_paths);
        let extension_prefix = self
            .extension_prefix
            .map(|ext| format!(".{ext}"))
            .unwrap_or_default();

        for (declaration, class, casm_class) in izip!(contracts, classes, casm_classes) {
            let contract_name = declaration.submodule_id.name(db.upcast_mut());
            let contract_path = declaration.module_id().full_path(db.upcast_mut());

            let contract_selector = ContractSelector(contract_path);
            let package_name = contract_selector.package();
            let contract_stem = file_stem_calculator.get_stem(contract_selector.full_path());

            let file_stem = format!("{}_{contract_stem}", self.target_name);

            let mut artifact = ContractArtifacts::new(
                package_name,
                &contract_name,
                contract_selector.full_path().as_str(),
                &declaration.module_id().full_path(db.upcast_mut()),
            );

            if self.sierra {
                let file_name = format!("{file_stem}{extension_prefix}.contract_class.json");
                write_json(&file_name, "output file", &self.target_dir, ws, class)?;
                artifact.artifacts.sierra = Some(file_name);
            }

            if self.casm {
                if let Some(casm_class) = casm_class {
                    let file_name =
                        format!("{file_stem}{extension_prefix}.compiled_contract_class.json");
                    write_json(&file_name, "output file", &self.target_dir, ws, casm_class)?;
                    artifact.artifacts.casm = Some(file_name);
                }
            }

            artifacts.contracts.push(artifact);
        }

        artifacts.finish();

        write_json(
            &format!(
                "{}{extension_prefix}.starknet_artifacts.json",
                self.target_name
            ),
            "starknet artifacts file",
            &self.target_dir,
            ws,
            &artifacts,
        )?;

        Ok(())
    }
}
