use console::{pad_str, Alignment, Style};
use serde::{Serialize, Serializer};

use crate::Message;

/// Indication of starting or finishing of a significant process in the application.
///
/// The `status` part categorizes the process, and should always be a single verb, for example:
/// _Compiling_, _Running_.
/// In text mode, status messages are coloured and right-padded for better aesthetics.
/// Padding is hardcoded to **12** characters, therefore avoid using words longer than
/// **11** characters.
/// The `message` part is a free-form text describing the details of what's going on.
#[derive(Serialize)]
pub struct Status<'a> {
    status: &'a str,
    #[serde(skip)]
    color: &'a str,
    message: &'a str,
}

impl<'a> Status<'a> {
    /// Create a new [`Status`] with default color (green).
    pub fn new(status: &'a str, message: &'a str) -> Self {
        Self::with_color(status, "green", message)
    }

    /// Create a new [`Status`] with the given color.
    pub fn with_color(status: &'a str, color: &'a str, message: &'a str) -> Self {
        Self {
            status,
            color,
            message,
        }
    }
}

impl Message for Status<'_> {
    fn text(self) -> String {
        format!(
            "{} {}",
            Style::from_dotted_str(self.color).bold().apply_to(pad_str(
                self.status,
                12,
                Alignment::Right,
                None,
            )),
            self.message
        )
    }

    fn structured<S: Serializer>(self, ser: S) -> Result<S::Ok, S::Error> {
        let status = self.status.to_lowercase();
        Status {
            status: &status,
            color: self.color,
            message: self.message,
        }
        .serialize(ser)
    }
}
