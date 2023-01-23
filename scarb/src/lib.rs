//! Scarb is the build tool and package manager for the [Cairo] programming language.
//!
//! [cairo]: https://cairo-lang.org/

#![deny(rustdoc::broken_intra_doc_links)]
#![deny(rustdoc::private_intra_doc_links)]
#![warn(rust_2018_idioms)]

pub mod core;
pub mod dirs;
pub mod flock;
mod internal;
pub mod metadata;
pub mod ops;
mod resolver;
mod sources;
pub mod ui;

pub const SCARB_ENV: &str = "SCARB";

pub const MANIFEST_FILE_NAME: &str = "Scarb.toml";

pub const DEFAULT_SOURCE_DIR_NAME: &str = "src";
pub const DEFAULT_TARGET_DIR_NAME: &str = "target";
