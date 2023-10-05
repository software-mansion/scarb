//! This module provides various ready to use message types and widgets for use with
//! a [`Ui`][crate::Ui].

pub use machine::*;
pub use spinner::*;
pub use status::*;
pub use typed::*;
pub use value::*;

mod machine;
mod spinner;
mod status;
mod typed;
mod value;
