use clap::ValueEnum;

pub use machine::*;
pub use message::*;
pub use spinner::*;
pub use status::*;
pub use typed::*;
pub use value::*;
pub use widget::*;

mod machine;
mod message;
mod spinner;
mod status;
mod typed;
mod value;
mod widget;

/// The requested verbosity of output.
///
/// # Ordering
/// [`Verbosity::Quiet`] < [`Verbosity::Normal`] < [`Verbosity::Verbose`]
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum Verbosity {
    Quiet,
    Normal,
    Verbose,
}

impl Default for Verbosity {
    fn default() -> Self {
        Self::Normal
    }
}

/// The requested format of output (either textual or JSON).
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
}

#[cfg(test)]
mod tests {
    use super::Verbosity;

    #[test]
    fn verbosity_ord() {
        use Verbosity::*;
        assert!(Quiet < Normal);
        assert!(Normal < Verbose);
    }
}
