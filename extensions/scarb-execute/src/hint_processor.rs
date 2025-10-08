use cairo_lang_casm::hints::{Hint, StarknetHint};
use cairo_lang_runner::casm_run::{
    MemBuffer, cell_ref_to_relocatable, extract_relocatable, vm_get_range,
};
use cairo_lang_runner::{CairoHintProcessor, insert_value_to_cellref};
use cairo_vm::Felt252;
use cairo_vm::hint_processor::hint_processor_definition::{HintProcessorLogic, HintReference};
use cairo_vm::serde::deserialize_program::ApTracking;
use cairo_vm::types::exec_scope::ExecutionScopes;
use cairo_vm::vm::errors::hint_errors::HintError;
use cairo_vm::vm::errors::vm_errors::VirtualMachineError;
use cairo_vm::vm::runners::cairo_runner::{ResourceTracker, RunResources};
use cairo_vm::vm::vm_core::VirtualMachine;
use scarb_oracle_hint_service::OracleHintService;
use std::any::Any;
use std::collections::HashMap;
use std::rc::Rc;

pub struct ExecuteHintProcessor<'a> {
    pub cairo_hint_processor: CairoHintProcessor<'a>,
    pub oracle_hint_service: OracleHintService,
}

impl<'a> HintProcessorLogic for ExecuteHintProcessor<'a> {
    fn execute_hint(
        &mut self,
        vm: &mut VirtualMachine,
        exec_scopes: &mut ExecutionScopes,
        hint_data: &Box<dyn Any>,
    ) -> Result<(), HintError> {
        if let Some(Hint::Starknet(StarknetHint::Cheatcode {
            selector,
            input_start,
            input_end,
            output_start,
            output_end,
        })) = hint_data.downcast_ref::<Hint>()
            && let selector = selector.value.to_bytes_be().1
            && let Some(oracle_selector) = self.oracle_hint_service.accept_cheatcode(&selector)
        {
            // Extract the inputs.
            let input_start = extract_relocatable(vm, input_start)?;
            let input_end = extract_relocatable(vm, input_end)?;
            let inputs = vm_get_range(vm, input_start, input_end)?;

            // Prepare output segment.
            let mut res_segment = MemBuffer::new_segment(vm);
            let res_segment_start = res_segment.ptr;

            // Execute the cheatcode.
            let output = self
                .oracle_hint_service
                .execute_cheatcode(oracle_selector, &inputs);
            res_segment.write_data(output.into_iter())?;

            // Store output and terminate execution.
            let res_segment_end = res_segment.ptr;
            insert_value_to_cellref!(vm, output_start, res_segment_start)?;
            insert_value_to_cellref!(vm, output_end, res_segment_end)?;

            Ok(())
        } else {
            self.cairo_hint_processor
                .execute_hint(vm, exec_scopes, hint_data)
        }
    }

    fn compile_hint(
        &self,
        hint_code: &str,
        ap_tracking_data: &ApTracking,
        reference_ids: &HashMap<String, usize>,
        references: &[HintReference],
        constants: Rc<HashMap<String, Felt252>>,
    ) -> Result<Box<dyn Any>, VirtualMachineError> {
        self.cairo_hint_processor.compile_hint(
            hint_code,
            ap_tracking_data,
            reference_ids,
            references,
            constants,
        )
    }
}

impl<'a> ResourceTracker for ExecuteHintProcessor<'a> {
    fn consumed(&self) -> bool {
        self.cairo_hint_processor.consumed()
    }

    fn consume_step(&mut self) {
        self.cairo_hint_processor.consume_step();
    }

    fn get_n_steps(&self) -> Option<usize> {
        self.cairo_hint_processor.get_n_steps()
    }

    fn run_resources(&self) -> &RunResources {
        self.cairo_hint_processor.run_resources()
    }
}
