use scarb_ui::Ui;
use std::sync::{LazyLock, Mutex};

// Scarb doc diagnostics storage.
pub(crate) struct ScarbDocDiagnostics {
    messages: Mutex<Vec<String>>,
}

impl ScarbDocDiagnostics {
    fn new() -> Self {
        ScarbDocDiagnostics {
            messages: Mutex::new(Vec::new()),
        }
    }

    fn add_message(&self, message: String) {
        let mut messages = self.messages.lock().unwrap();
        messages.push(message);
    }

    fn print_messages(&self, ui: &Ui) {
        let mut messages = self.messages.lock().unwrap();
        for message in messages.iter() {
            ui.warn(message);
        }
        messages.clear();
    }
}

static DIAGNOSTICS: LazyLock<ScarbDocDiagnostics> = LazyLock::new(ScarbDocDiagnostics::new);

pub fn add_diagnostic_message(message: String) {
    DIAGNOSTICS.add_message(message);
}

pub fn print_diagnostics(ui: &Ui) {
    DIAGNOSTICS.print_messages(ui);
}
