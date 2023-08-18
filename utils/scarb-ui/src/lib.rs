use clap::ValueEnum;
pub use indicatif::{
    BinaryBytes, DecimalBytes, FormattedDuration, HumanBytes, HumanCount, HumanDuration,
    HumanFloatCount,
};

pub use message::*;
pub use verbosity::*;
pub use widget::*;

use crate::components::TypedMessage;

pub mod args;
pub mod components;
mod message;
mod verbosity;
mod widget;

/// The requested format of output (either textual or JSON).
#[derive(Copy, Clone, Debug, Default, Eq, PartialEq, ValueEnum)]
pub enum OutputFormat {
    #[default]
    Text,
    Json,
}

/// An abstraction around console output which stores preferences for output format (human vs JSON),
/// colour, etc.
///
/// All human-oriented messaging (basically all writes to `stdout`) must go through this object.
#[derive(Debug)]
pub struct Ui {
    verbosity: Verbosity,
    output_format: OutputFormat,
}

impl Ui {
    pub fn new(verbosity: Verbosity, output_format: OutputFormat) -> Self {
        Self {
            verbosity,
            output_format,
        }
    }

    pub fn verbosity(&self) -> Verbosity {
        self.verbosity
    }

    pub fn output_format(&self) -> OutputFormat {
        self.output_format
    }

    pub fn print<T: Message>(&self, message: T) {
        if self.verbosity >= Verbosity::Normal {
            self.do_print(message);
        }
    }

    pub fn verbose<T: Message>(&self, message: T) {
        if self.verbosity >= Verbosity::Verbose {
            self.do_print(message);
        }
    }

    pub fn widget<T: Widget>(&self, widget: T) -> Option<T::Handle> {
        if self.output_format == OutputFormat::Text && self.verbosity >= Verbosity::Normal {
            let handle = widget.text();
            Some(handle)
        } else {
            None
        }
    }

    pub fn warn(&self, message: impl AsRef<str>) {
        self.print(TypedMessage::styled("warn", "yellow", message.as_ref()))
    }

    pub fn error(&self, message: impl AsRef<str>) {
        self.print(TypedMessage::styled("error", "red", message.as_ref()))
    }

    pub fn anyhow(&self, error: &anyhow::Error) {
        // NOTE: Some errors, particularly ones from `toml_edit` like to add trailing newlines.
        //   This isn't a big problem for users, but it's causing issues in tests, where trailing
        //   whitespace collides with `indoc`.
        self.error(format!("{error:?}").trim())
    }

    fn do_print<T: Message>(&self, message: T) {
        match self.output_format {
            OutputFormat::Text => message.print_text(),
            OutputFormat::Json => message.print_json(),
        }
    }

    /// Forces colorization on or off for stdout.
    pub fn force_colors_enabled(&self, enable: bool) {
        console::set_colors_enabled(enable);
    }
}
