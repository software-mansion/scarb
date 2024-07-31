use serde::Serializer;

#[cfg(doc)]
use super::Ui;

const JSON_SKIP_MESSAGE: &str = "UI_INTERNAL_SKIP";

/// A typed object that can be either printed as a human-readable message or serialized as JSON.
///
/// The [`TypedMessage`][crate::components::TypedMessage] and [`Status`][crate::components::Status]
/// structs are the most frequently used kinds of messages.
pub trait Message {
    // NOTE: The `print_*` low-level methods functions are doc hidden,
    //   because they are not considered stable.

    /// Return textual representation of this message.
    ///
    /// Default implementation returns empty string, making [`Ui`] skip printing this message.
    fn text(self) -> String
    where
        Self: Sized,
    {
        String::new()
    }

    #[doc(hidden)]
    fn print_text(self)
    where
        Self: Sized,
    {
        let text = self.text();
        if !text.is_empty() {
            println!("{text}");
        }
    }

    /// Serialize this structured message to a serializer which is routed to [`Ui`] output stream.
    ///
    /// Default implementation does not serialize anything, making [`Ui`] skip printing
    /// this message.
    fn structured<S: Serializer>(self, ser: S) -> Result<S::Ok, S::Error>
    where
        Self: Sized,
    {
        Self::skip_structured(ser)
    }

    #[doc(hidden)]
    fn skip_structured<S: Serializer>(_ser: S) -> Result<S::Ok, S::Error> {
        Err(serde::ser::Error::custom(JSON_SKIP_MESSAGE))
    }

    #[doc(hidden)]
    fn print_json(self)
    where
        Self: Sized,
    {
        let mut buf = Vec::with_capacity(128);
        let mut serializer = serde_json::Serializer::new(&mut buf);
        match self.structured(&mut serializer) {
            Ok(_) => {
                let string = unsafe {
                    // UNSAFE: JSON is always UTF-8 encoded.
                    String::from_utf8_unchecked(buf)
                };
                println!("{string}");
            }
            Err(err) => {
                if err.to_string() != JSON_SKIP_MESSAGE {
                    panic!("JSON serialization of UI message must not fail: {err}")
                }
            }
        }
    }
}

impl Message for &str {
    fn text(self) -> String {
        self.to_string()
    }
}

impl Message for String {
    fn text(self) -> String {
        self
    }
}
