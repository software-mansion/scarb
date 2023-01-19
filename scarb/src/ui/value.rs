use std::fmt::Display;

use serde::{Serialize, Serializer};
use serde_json::json;

use crate::ui::Message;

pub struct ValueMessage<'a, T> {
    name: &'a str,
    value: &'a T,
}

impl<'a, T> ValueMessage<'a, T> {
    pub fn new(name: &'a str, value: &'a T) -> Self {
        Self { name, value }
    }
}

impl<'a, T> Message for ValueMessage<'a, T>
where
    T: Display + Serialize,
{
    fn text(self) -> String
    where
        Self: Sized,
    {
        self.value.to_string()
    }

    fn structured<S: Serializer>(self, ser: S) -> Result<S::Ok, S::Error>
    where
        Self: Sized,
    {
        json!({
            self.name: self.value
        })
        .serialize(ser)
    }
}
