//! Terminal user interface primitives used by [Scarb] and its extensions.
//!
//! This crate focuses mainly on two areas:
//!
//! 1. [`Ui`] and [`components`]: Serving a unified interface for communication with the user,
//!    either via:
//!     - rendering human-readable messages or interactive widgets,
//!     - or printing machine-parseable JSON-NL messages, depending on runtime configuration.
//! 2. [`args`]: Providing reusable [`clap`] arguments for common tasks.
//!
//! There are also re-export from various TUI crates recommended for use in Scarb ecosystem,
//! such as [`indicatif`] or [`console`].
//!
//! [scarb]: https://docs.swmansion.com/scarb

#![deny(clippy::dbg_macro)]
#![deny(clippy::disallowed_methods)]
#![deny(missing_docs)]
#![deny(rustdoc::broken_intra_doc_links)]
#![deny(rustdoc::missing_crate_level_docs)]
#![deny(rustdoc::private_intra_doc_links)]
#![warn(rust_2018_idioms)]

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
    /// Render human-readable messages and interactive widgets.
    #[default]
    Text,
    /// Render machine-parseable JSON-NL messages.
    Json,
}

/// An abstraction around console output which stores preferences for output format (human vs JSON),
/// colour, etc.
///
/// All human-oriented messaging (basically all writes to `stdout`) must go through this object.
#[derive(Clone, Debug)]
pub struct Ui {
    verbosity: Verbosity,
    output_format: OutputFormat,
}

impl Ui {
    /// Create a new [`Ui`] instance configured with the given verbosity and output format.
    pub fn new(verbosity: Verbosity, output_format: OutputFormat) -> Self {
        Self {
            verbosity,
            output_format,
        }
    }

    /// Get the verbosity level of this [`Ui`] instance.
    pub fn verbosity(&self) -> Verbosity {
        self.verbosity
    }

    /// Get the output format of this [`Ui`] instance.
    pub fn output_format(&self) -> OutputFormat {
        self.output_format
    }

    /// Print the message to standard output if not in quiet verbosity mode.
    pub fn print<T: Message>(&self, message: T) {
        if self.verbosity >= Verbosity::Normal {
            self.do_print(message);
        }
    }

    /// Print the message to the standard output only in verbose mode.
    pub fn verbose<T: Message>(&self, message: T) {
        if self.verbosity >= Verbosity::Verbose {
            self.do_print(message);
        }
    }

    /// Display an interactive widget and return a handle for further interaction.
    ///
    /// The widget will be only displayed if not in quiet mode, and if the output format is text.
    pub fn widget<T: Widget>(&self, widget: T) -> Option<T::Handle> {
        if self.output_format == OutputFormat::Text && self.verbosity >= Verbosity::Normal {
            let handle = widget.text();
            Some(handle)
        } else {
            None
        }
    }

    /// Print a warning to the user.
    pub fn warn(&self, message: impl AsRef<str>) {
        self.print(TypedMessage::styled("warn", "yellow", message.as_ref()))
    }

    /// Print an error to the user.
    pub fn error(&self, message: impl AsRef<str>) {
        self.print(TypedMessage::styled("error", "red", message.as_ref()))
    }

    /// Nicely format an [`anyhow::Error`] for display to the user, and print it with [`Ui::error`].
    pub fn anyhow(&self, error: &anyhow::Error) {
        // NOTE: Some errors, particularly ones from `toml_edit` like to add trailing newlines.
        //   This isn't a big problem for users, but it's causing issues in tests, where trailing
        //   whitespace collides with `indoc`.
        self.error(format!("{error:?}").trim())
    }

    /// Nicely format an [`anyhow::Error`] for display to the user, and print it with [`Ui::warn`].
    pub fn warn_anyhow(&self, error: &anyhow::Error) {
        // NOTE: Some errors, particularly ones from `toml_edit` like to add trailing newlines.
        //   This isn't a big problem for users, but it's causing issues in tests, where trailing
        //   whitespace collides with `indoc`.
        self.warn(format!("{error:?}").trim())
    }

    fn do_print<T: Message>(&self, message: T) {
        match self.output_format {
            OutputFormat::Text => message.print_text(),
            OutputFormat::Json => message.print_json(),
        }
    }

    /// Forces colorization on or off for stdout.
    ///
    /// This overrides the default for the current process and changes the return value of
    /// the [`Ui::has_colors_enabled`] function.
    pub fn force_colors_enabled(&self, enable: bool) {
        console::set_colors_enabled(enable);
    }

    /// Returns `true` if colors should be enabled for stdout.
    ///
    /// This honors the [clicolors spec](http://bixense.com/clicolors/).
    ///
    /// * `CLICOLOR != 0`: ANSI colors are supported and should be used when the program isn't piped.
    /// * `CLICOLOR == 0`: Don't output ANSI color escape codes.
    /// * `CLICOLOR_FORCE != 0`: ANSI colors should be enabled no matter what.
    pub fn has_colors_enabled(&self) -> bool {
        console::colors_enabled()
    }

    /// Forces colorization on or off for stdout.
    ///
    /// This overrides the default for the current process and changes the return value of
    /// the [`Ui::has_colors_enabled`] function.
    pub fn force_colors_enabled_stderr(&self, enable: bool) {
        console::set_colors_enabled_stderr(enable);
    }

    /// Returns `true` if colors should be enabled for stderr.
    ///
    /// This honors the [clicolors spec](http://bixense.com/clicolors/).
    ///
    /// * `CLICOLOR != 0`: ANSI colors are supported and should be used when the program isn't piped.
    /// * `CLICOLOR == 0`: Don't output ANSI color escape codes.
    /// * `CLICOLOR_FORCE != 0`: ANSI colors should be enabled no matter what.
    pub fn has_colors_enabled_stderr(&self) -> bool {
        console::colors_enabled_stderr()
    }
}
