use std::fmt::Display;

use serde::{Serialize, Serializer};
use serde_json::json;

use crate::Message;

/// Print a single value result of a computation to the user.
///
/// In JSON mode, this will emit like this:
/// ```json
/// {"name":value}
/// ```
///
/// In text mode, `name` is omitted.
pub struct ValueMessage<'a, T> {
    name: &'a str,
    value: &'a T,
}

impl<'a, T> ValueMessage<'a, T> {
    /// Create a new value message.
    pub fn new(name: &'a str, value: &'a T) -> Self {
        Self { name, value }
    }
}

impl<T> Message for ValueMessage<'_, T>
where
    T: Display + Serialize,
{
    fn text(self) -> String {
        self.value.to_string()
    }

    fn structured<S: Serializer>(self, ser: S) -> Result<S::Ok, S::Error> {
        json!({
            self.name: self.value
        })
        .serialize(ser)
    }
}
