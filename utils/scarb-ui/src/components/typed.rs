use console::Style;
use serde::{Serialize, Serializer};

use crate::Message;

/// Generic textual message with _type_ prefix.
///
/// Use this message type to display any kinds of warnings, errors etc.
/// The type prefix can be stylized in text mode.
#[derive(Serialize)]
pub struct TypedMessage<'a> {
    r#type: &'a str,
    message: &'a str,

    #[serde(skip)]
    type_style: Option<&'a str>,
    #[serde(skip)]
    skip_type_for_text: bool,
}

impl<'a> TypedMessage<'a> {
    /// Create a message with the given type, its style and the message text proper.
    pub fn styled(ty: &'a str, type_style: &'a str, message: &'a str) -> Self {
        Self {
            r#type: ty,
            message,
            type_style: Some(type_style),
            skip_type_for_text: false,
        }
    }

    /// Create a message that does not print type prefix in text mode.
    ///
    /// ## Example
    /// Scarb uses this for emitting Cairo compiler diagnostics.
    /// In text mode it prints the diagnostic as-is, while in JSON mode it wraps it as:
    /// ```json
    /// {"type":"diagnostic","message":"<diagnostic>"}
    /// ```
    pub fn naked_text(ty: &'a str, message: &'a str) -> Self {
        Self {
            r#type: ty,
            message,
            type_style: None,
            skip_type_for_text: true,
        }
    }
}

impl<'a> Message for TypedMessage<'a> {
    fn text(self) -> String {
        if self.skip_type_for_text {
            self.message.to_string()
        } else {
            format!(
                "{}: {}",
                self.type_style
                    .map(Style::from_dotted_str)
                    .unwrap_or_else(Style::new)
                    .apply_to(self.r#type),
                self.message
            )
        }
    }

    fn structured<S: Serializer>(self, ser: S) -> Result<S::Ok, S::Error> {
        self.serialize(ser)
    }
}
