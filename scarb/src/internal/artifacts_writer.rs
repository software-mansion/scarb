use crate::compiler::helpers::{write_json, write_json_with_byte_count, write_string};
use crate::compiler::{
    MAX_CASM_PROGRAM_FELTS, MAX_COMPILED_CONTRACT_CLASS_BYTES, MAX_CONTRACT_CLASS_BYTES,
    MAX_SIERRA_PROGRAM_FELTS,
};
use crate::core::Workspace;
use crate::flock::Filesystem;
use anyhow::{Context, Result};
use cairo_lang_sierra::program::{ProgramArtifact, VersionedProgram};
use cairo_lang_starknet_classes::abi::Contract;
use cairo_lang_starknet_classes::casm_contract_class::CasmContractClass;
use cairo_lang_starknet_classes::contract_class::{ContractClass, ContractEntryPoints};
use cairo_lang_utils::bigint::BigUintAsHex;
use indoc::formatdoc;
use serde::Serialize;
use std::sync::{Arc, mpsc};
use std::{mem, thread};
use tracing::trace_span;

pub enum Request {
    ProgramArtifact {
        file: File,
        value: Arc<ProgramArtifact>,
    },
    ProgramArtifactText {
        file: File,
        value: Arc<ProgramArtifact>,
    },
    ContractClassArtifact {
        file: File,
        value: Arc<ContractClass>,
        contract_stem: String,
    },
    CasmContractClassArtifact {
        file: File,
        value: Arc<CasmContractClass>,
        contract_stem: String,
    },
}

pub struct File {
    pub file_name: String,
    pub description: String,
    pub target_dir: Filesystem,
}

pub struct ArtifactsWriter {
    handle: Option<thread::JoinHandle<Result<()>>>,
}

impl ArtifactsWriter {
    pub fn new(request_stream: mpsc::Receiver<Request>, ws: &Workspace<'_>) -> Self {
        let ws = unsafe {
            // This should be safe, as we know we will join the artifact writer thread before
            // the workspace is dropped.
            mem::transmute::<&Workspace<'_>, &Workspace<'_>>(ws)
        };
        let handle = thread::Builder::new()
            .name("scarb-artifacts-writer".into())
            .spawn(move || {
                let span = trace_span!("writer requests");
                for request in request_stream.iter() {
                    let _guard = span.enter();
                    handle_request(request, ws)
                        .with_context(|| "failed to handle artifact writer request")?;
                }
                Ok(())
            })
            .expect("failed to spawn artifacts writer thread");
        Self {
            handle: Some(handle),
        }
    }

    pub fn join(mut self) -> Result<()> {
        let result = if let Some(handle) = self.handle.take() {
            handle
                .join()
                .expect("failed to join artifacts writer thread")
        } else {
            Ok(())
        };
        mem::forget(self); // Defuse the drop bomb.
        result
    }
}

impl Drop for ArtifactsWriter {
    fn drop(&mut self) {
        panic!("not defused: ArtifactsWriter dropped without join");
    }
}

fn handle_request(request: Request, ws: &Workspace<'_>) -> Result<()> {
    match request {
        Request::ProgramArtifact {
            file:
                File {
                    file_name,
                    description,
                    target_dir,
                },
            value,
        } => {
            // Cloning the underlying program is expensive, but we can afford it here,
            // as we are on a dedicated thread anyway.
            let sierra_program: VersionedProgram = value.as_ref().clone().into();
            write_json(
                file_name.as_str(),
                description.as_str(),
                &target_dir,
                ws,
                &sierra_program,
            )?;
        }
        Request::ProgramArtifactText {
            file:
                File {
                    file_name,
                    description,
                    target_dir,
                },
            value,
        } => {
            // vide supra
            let sierra_program: VersionedProgram = value.as_ref().clone().into();
            write_string(
                file_name.as_str(),
                description.as_str(),
                &target_dir,
                ws,
                &sierra_program,
            )?;
        }
        Request::ContractClassArtifact {
            file,
            value,
            contract_stem,
        } => write_contract_class(file, value, contract_stem, ws)?,
        Request::CasmContractClassArtifact {
            file,
            value,
            contract_stem,
        } => write_casm_contract_class(file, value, contract_stem, ws)?,
    }
    Ok(())
}

fn write_contract_class(
    File {
        file_name,
        description,
        target_dir,
    }: File,
    class: Arc<ContractClass>,
    contract_stem: String,
    ws: &Workspace<'_>,
) -> Result<()> {
    let sierra_felts = class.sierra_program.len();
    if sierra_felts > MAX_SIERRA_PROGRAM_FELTS {
        ws.config().ui().warn(formatdoc! {r#"
                Sierra program exceeds maximum byte-code size on Starknet for contract `{}`:
                {MAX_SIERRA_PROGRAM_FELTS} felts allowed. Actual size: {sierra_felts} felts.
            "#, contract_stem.clone()});
    }
    let class_size =
        write_json_with_byte_count(&file_name, &description, &target_dir, ws, class.clone())?;
    if class_size > MAX_CONTRACT_CLASS_BYTES {
        // Debug info is omitted on Starknet.
        // Only warn if size without debug info exceeds the limit as well.
        let rpc_class = ContractClassNoDebug::new(class.as_ref());
        let rpc_class_size = serde_json::to_vec(&rpc_class)?.len();

        if rpc_class_size > MAX_CONTRACT_CLASS_BYTES {
            ws.config().ui().warn(formatdoc! {r#"
                    Contract class size exceeds maximum allowed size on Starknet for contract `{}`:
                    {MAX_CONTRACT_CLASS_BYTES} bytes allowed. Actual size (without debug info): {rpc_class_size} bytes.
                "#, contract_stem.clone()});
        }
    }

    Ok(())
}

// Represents a contract in the Starknet network as defined in Starknet JSON-RPC spec:
// https://github.com/starkware-libs/starknet-specs/blob/2030a650be4e40cfa34d5051a0334f375384a421/api/starknet_api_openrpc.json#L3030
#[derive(Clone, Debug, Serialize)]
struct ContractClassNoDebug<'a> {
    sierra_program: &'a Vec<BigUintAsHex>,
    contract_class_version: &'a String,
    entry_points_by_type: &'a ContractEntryPoints,
    abi: &'a Option<Contract>,
}

impl<'a> ContractClassNoDebug<'a> {
    fn new(contract_class: &'a ContractClass) -> Self {
        Self {
            sierra_program: &contract_class.sierra_program,
            contract_class_version: &contract_class.contract_class_version,
            entry_points_by_type: &contract_class.entry_points_by_type,
            abi: &contract_class.abi,
        }
    }
}

fn write_casm_contract_class(
    File {
        file_name,
        description,
        target_dir,
    }: File,
    casm_class: Arc<CasmContractClass>,
    contract_stem: String,
    ws: &Workspace<'_>,
) -> Result<()> {
    let casm_felts = casm_class.bytecode.len();
    if casm_felts > MAX_CASM_PROGRAM_FELTS {
        ws.config().ui().warn(formatdoc! {r#"
                CASM program exceeds maximum byte-code size on Starknet for contract `{}`:
                {MAX_CASM_PROGRAM_FELTS} felts allowed. Actual size: {casm_felts} felts.
            "#, contract_stem.clone()});
    }

    let compiled_class_size =
        write_json_with_byte_count(&file_name, &description, &target_dir, ws, casm_class)?;
    if compiled_class_size > MAX_COMPILED_CONTRACT_CLASS_BYTES {
        ws.config().ui().warn(formatdoc! {r#"
                Compiled contract class size exceeds maximum allowed size on Starknet for contract `{}`:
                {MAX_COMPILED_CONTRACT_CLASS_BYTES} bytes allowed. Actual size: {compiled_class_size} bytes.
            "#, contract_stem.clone()});
    }
    Ok(())
}
