//! This module provides various ready to use message types and widgets for use with
//! a [`Ui`][crate::Ui].

pub use machine::*;
pub use new_line::*;
pub use spinner::*;
pub use status::*;
pub use test_result::*;
pub use typed::*;
pub use value::*;

mod machine;
mod new_line;
mod spinner;
mod status;
mod test_result;
mod typed;
mod value;
