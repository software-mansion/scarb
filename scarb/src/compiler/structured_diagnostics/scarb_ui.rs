use scarb_ui::components::MachineMessage;

use super::core::{StructuredDiagnosticMessage, StructuredDiagnosticsSink};
use crate::core::MachineDiagnosticSeverity;

pub struct ScarbUiStructuredDiagnosticsSink {
    ui: scarb_ui::Ui,
}

impl ScarbUiStructuredDiagnosticsSink {
    pub fn new(ui: scarb_ui::Ui) -> Self {
        Self { ui }
    }
}

impl StructuredDiagnosticsSink for ScarbUiStructuredDiagnosticsSink {
    fn emit(&mut self, message: StructuredDiagnosticMessage) {
        let severity = message.severity();
        match severity {
            MachineDiagnosticSeverity::Error => self.ui.record_error(),
            MachineDiagnosticSeverity::Warning => self.ui.record_warning(),
        }
        self.ui.print(MachineMessage(message));
    }
}
