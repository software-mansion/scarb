use crate::output::ExecutionResources;
use anyhow::{Result, anyhow};
use cairo_annotations::trace_data::{
    CairoExecutionInfo, CallEntryPoint as ProfilerCallEntryPoint, CallTraceV1 as ProfilerCallTrace,
    CallType as ProfilerCallType, CasmLevelInfo, ContractAddress, EntryPointSelector,
    EntryPointType as ProfilerEntryPointType, ExecutionResources as ProfilerExecutionResources,
    TraceEntry as ProfilerTraceEntry,
};
use cairo_vm::vm::trace::trace_entry::RelocatedTraceEntry;
use camino::Utf8PathBuf;
use scarb_extensions_cli::execute::ExecutionTarget;
use scarb_metadata::PackageMetadata;

#[derive(Default, Debug)]
pub enum TrackedResource {
    #[default]
    CairoSteps,
    SierraGas,
}

impl From<&str> for TrackedResource {
    fn from(m: &str) -> Self {
        match m {
            "sierra-gas" => TrackedResource::SierraGas,
            _ => TrackedResource::CairoSteps,
        }
    }
}

impl From<TrackedResource> for &str {
    fn from(r: TrackedResource) -> Self {
        match r {
            TrackedResource::SierraGas => "sierra-gas",
            TrackedResource::CairoSteps => "cairo-steps",
        }
    }
}

pub fn build_profiler_call_trace(
    target: &ExecutionTarget,
    vm_trace: Option<Vec<RelocatedTraceEntry>>,
    vm_resources: ExecutionResources,
    tracked_resource: &TrackedResource,
    source_sierra_path: Utf8PathBuf,
    function_name: Option<String>,
    program_offset: usize,
) -> Result<ProfilerCallTrace> {
    let entry_point = build_profiler_call_entry_point(target, function_name);
    let profiler_vm_trace = vm_trace
        .as_ref()
        .map(|trace_data| trace_data.iter().map(build_profiler_trace_entry).collect())
        .unwrap();
    let cairo_execution_info = CairoExecutionInfo {
        casm_level_info: CasmLevelInfo {
            run_with_call_header: true,
            vm_trace: profiler_vm_trace,
            program_offset: Some(program_offset),
        },
        source_sierra_path,
        enable_gas: Some(false),
    };

    let mut execution_resources: ProfilerExecutionResources =
        vm_resources.try_into().map_err(|e: String| anyhow!(e))?;

    // The `CallTraceV1` trace format defines `vm_resources` and `gas_consumed` used by cairo-profiler to determine what resource was tracked.
    // Exactly one of the two should be non-zero - indicating the resource that has been tracked, while the other should be reset.
    // Since we cannot infer the tracked resources from execution directly, we mask one of the fields based on user provided configuration.
    // This maintains compatibility with other execution environments that produce traces in `CallTraceV1` format.
    match tracked_resource {
        TrackedResource::CairoSteps => execution_resources.gas_consumed = None,
        TrackedResource::SierraGas => execution_resources.vm_resources = Default::default(),
    };

    Ok(ProfilerCallTrace {
        entry_point,
        cumulative_resources: execution_resources,
        used_l1_resources: Default::default(),
        nested_calls: Default::default(),
        cairo_execution_info: Some(cairo_execution_info),
    })
}

pub fn get_profiler_tracked_resource(package_metadata: &PackageMetadata) -> TrackedResource {
    let tracked_resource = package_metadata
        .tool_metadata("cairo-profiler")
        .and_then(|val| val.get("tracked-resource"))
        .and_then(|val| val.as_str())
        .unwrap_or("cairo-steps");

    TrackedResource::from(tracked_resource)
}

fn build_profiler_call_entry_point(
    target: &ExecutionTarget,
    function_name: Option<String>,
) -> ProfilerCallEntryPoint {
    ProfilerCallEntryPoint {
        class_hash: None,
        entry_point_type: ProfilerEntryPointType::External,
        entry_point_selector: EntryPointSelector::default(),
        contract_address: ContractAddress::default(),
        call_type: ProfilerCallType::Call,
        contract_name: Some(format!("SCARB_EXECUTE\nTarget: {}", target)),
        function_name,
        calldata_len: None,
        events_summary: None,
        signature_len: None,
    }
}

fn build_profiler_trace_entry(value: &RelocatedTraceEntry) -> ProfilerTraceEntry {
    ProfilerTraceEntry {
        pc: value.pc,
        ap: value.ap,
        fp: value.fp,
    }
}
