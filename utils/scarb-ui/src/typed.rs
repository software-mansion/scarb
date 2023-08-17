use console::Style;
use serde::{Serialize, Serializer};

use crate::Message;

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
    pub fn plain(ty: &'a str, message: &'a str) -> Self {
        Self {
            r#type: ty,
            message,
            type_style: None,
            skip_type_for_text: false,
        }
    }

    pub fn styled(ty: &'a str, type_style: &'a str, message: &'a str) -> Self {
        Self {
            r#type: ty,
            message,
            type_style: Some(type_style),
            skip_type_for_text: false,
        }
    }

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
