//! Scarb is the build tool and package manager for the [Cairo] programming language.
//!
//! [cairo]: https://cairo-lang.org/

#![deny(rustdoc::broken_intra_doc_links)]
#![deny(rustdoc::private_intra_doc_links)]
#![warn(rust_2018_idioms)]

pub use subcommands::EXTERNAL_CMD_PREFIX;

pub mod compiler;
pub mod core;
pub mod flock;
mod internal;
pub mod manifest_editor;
pub mod ops;
pub mod process;
mod resolver;
mod sources;
mod subcommands;
pub mod version;

pub const SCARB_ENV: &str = "SCARB";
pub const MANIFEST_FILE_NAME: &str = "Scarb.toml";
pub const DEFAULT_SOURCE_PATH: &str = "src/lib.cairo";
pub const DEFAULT_TARGET_DIR_NAME: &str = "target";
