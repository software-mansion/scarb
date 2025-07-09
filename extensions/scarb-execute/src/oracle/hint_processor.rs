use cairo_lang_casm::hints::{Hint, StarknetHint};
use cairo_lang_casm::operand::{CellRef, ResOperand};
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
use std::any::Any;
use std::collections::HashMap;

pub struct OracleHintProcessor<'a> {
    pub cairo_hint_processor: CairoHintProcessor<'a>,
    /// Whether `--experimental-oracles` flag has been enabled.
    experiment_enabled: bool,
}

enum MySelector {
    OracleInvoke,
}

impl MySelector {
    fn from_str(selector: &str) -> Option<Self> {
        match selector {
            "oracle_invoke" => Some(Self::OracleInvoke),
            _ => None,
        }
    }
}

struct MyCheatcode<'a> {
    selector: MySelector,
    input_start: &'a ResOperand,
    input_end: &'a ResOperand,
    output_start: &'a CellRef,
    output_end: &'a CellRef,
}

impl<'a> OracleHintProcessor<'a> {
    /// Creates a new instance of [`OracleHintProcessor`].
    pub fn new(cairo_hint_processor: CairoHintProcessor<'a>, experiment_enabled: bool) -> Self {
        Self {
            cairo_hint_processor,
            experiment_enabled,
        }
    }

    /// Gracefully look if this is one of the cheat codes supported by us.
    /// This function prepares context for proper hint execution.
    fn hijack_my_cheatcode<'b>(&self, hint_data: &'b Box<dyn Any>) -> Option<MyCheatcode<'b>> {
        if let Some(Hint::Starknet(StarknetHint::Cheatcode {
            selector,
            input_start,
            input_end,
            output_start,
            output_end,
        })) = hint_data.downcast_ref::<Hint>()
            && let Ok(selector) = str::from_utf8(&selector.value.to_bytes_be().1)
            && let Some(selector) = MySelector::from_str(selector)
        {
            Some(MyCheatcode {
                selector,
                input_start,
                input_end,
                output_start,
                output_end,
            })
        } else {
            None
        }
    }

    /// Take over execution just right when we learn this is "our" cheat code.
    fn execute_my_cheatcode(
        &mut self,
        vm: &mut VirtualMachine,
        MyCheatcode {
            selector,
            input_start,
            input_end,
            output_start,
            output_end,
        }: MyCheatcode<'_>,
    ) -> Result<(), HintError> {
        if !self.experiment_enabled {
            return Err(HintError::AssertionFailed(
                "Oracles are experimental feature. \
                    To enable, pass --experimental-oracles CLI flag."
                    .into(),
            ));
        }

        // Extract the inputs.
        let input_start = extract_relocatable(vm, input_start)?;
        let input_end = extract_relocatable(vm, input_end)?;
        let inputs = vm_get_range(vm, input_start, input_end)?;

        // Prepare output segment.
        let mut res_segment = MemBuffer::new_segment(vm);
        let res_segment_start = res_segment.ptr;

        // Route selector to particular execution methods.
        match selector {
            MySelector::OracleInvoke => self.execute_invoke(inputs, &mut res_segment)?,
        };

        // Store output and terminate execution.
        let res_segment_end = res_segment.ptr;
        insert_value_to_cellref!(vm, output_start, res_segment_start)?;
        insert_value_to_cellref!(vm, output_end, res_segment_end)?;

        Ok(())
    }

    /// Execute the `oracle_invoke` cheat code.
    fn execute_invoke(
        &mut self,
        _inputs: Vec<Felt252>,
        res_segment: &mut MemBuffer,
    ) -> Result<(), HintError> {
        let response: Vec<Felt252> = vec![]; // TODO
        res_segment.write_data(response.into_iter())?;
        Ok(())
    }
}

impl<'a> HintProcessorLogic for OracleHintProcessor<'a> {
    fn execute_hint(
        &mut self,
        vm: &mut VirtualMachine,
        exec_scopes: &mut ExecutionScopes,
        hint_data: &Box<dyn Any>,
        constants: &HashMap<String, Felt252>,
    ) -> Result<(), HintError> {
        if let Some(cheatcode) = self.hijack_my_cheatcode(hint_data) {
            self.execute_my_cheatcode(vm, cheatcode)
        } else {
            self.cairo_hint_processor
                .execute_hint(vm, exec_scopes, hint_data, constants)
        }
    }

    fn compile_hint(
        &self,
        hint_code: &str,
        ap_tracking_data: &ApTracking,
        reference_ids: &HashMap<String, usize>,
        references: &[HintReference],
    ) -> Result<Box<dyn Any>, VirtualMachineError> {
        self.cairo_hint_processor.compile_hint(
            hint_code,
            ap_tracking_data,
            reference_ids,
            references,
        )
    }
}

impl<'a> ResourceTracker for OracleHintProcessor<'a> {
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
