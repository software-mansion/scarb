use console::{pad_str, Alignment, Style};
use serde::{Serialize, Serializer};

use crate::Message;

/// Notes:
/// - `status` should always be a single verb, for example _Compiling_, _Running_.
#[derive(Serialize)]
pub struct Status<'a> {
    status: &'a str,
    #[serde(skip)]
    color: &'a str,
    message: &'a str,
}

impl<'a> Status<'a> {
    pub fn new(status: &'a str, message: &'a str) -> Self {
        Self::with_color(status, "green", message)
    }

    pub fn with_color(status: &'a str, color: &'a str, message: &'a str) -> Self {
        Self {
            status,
            color,
            message,
        }
    }
}

impl<'a> Message for Status<'a> {
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
