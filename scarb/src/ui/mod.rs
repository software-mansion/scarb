use clap::ValueEnum;
use console::Style;

pub use message::*;
pub use typed::*;

mod message;
mod typed;

#[derive(Copy, Clone, Debug, Eq, PartialEq, ValueEnum)]
pub enum OutputFormat {
    Text,
    Json,
}

impl Default for OutputFormat {
    fn default() -> Self {
        Self::Text
    }
}

/// An abstraction around console output which stores preferences for output format (human vs JSON),
/// colour, etc.
///
/// All human-oriented messaging (basically all writes to `stdout`) must go through this object.
#[derive(Debug)]
pub struct Ui {
    output_format: OutputFormat,
}

impl Ui {
    pub fn new(output_format: OutputFormat) -> Self {
        Self { output_format }
    }

    pub fn print(&self, message: impl Message) {
        match self.output_format {
            OutputFormat::Text => message.print_text(),
            OutputFormat::Json => message.print_json(),
        }
    }

    pub fn warn(&self, message: impl AsRef<str>) {
        self.print(TypedMessage::styled(
            "warn",
            &Style::new().yellow(),
            message.as_ref(),
        ))
    }

    pub fn error(&self, message: impl AsRef<str>) {
        self.print(TypedMessage::styled(
            "error",
            &Style::new().red(),
            message.as_ref(),
        ))
    }
}
