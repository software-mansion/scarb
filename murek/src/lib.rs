//! Scarb is the build tool and package manager for the [Cairo] programming language.
//!
//! [cairo]: https://cairo-lang.org/

#![deny(rustdoc::broken_intra_doc_links)]
#![deny(rustdoc::private_intra_doc_links)]
#![warn(rust_2018_idioms)]

pub mod core;
pub mod dirs;
mod internal;
pub mod metadata;
pub mod ops;
mod resolver;
mod sources;

pub const MUREK_ENV: &str = "MUREK";
