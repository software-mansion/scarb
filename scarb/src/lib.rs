//! Scarb is the build tool and package manager for the [Cairo] programming language.
//!
//! [cairo]: https://cairo-lang.org/

#![deny(clippy::dbg_macro)]
#![deny(clippy::disallowed_methods)]
#![deny(rustdoc::broken_intra_doc_links)]
#![deny(rustdoc::private_intra_doc_links)]
#![warn(rust_2018_idioms)]

use camino::Utf8PathBuf;
use std::sync::LazyLock;
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
pub const VCS_INFO_FILE_NAME: &str = "VCS.json";
pub const LOCK_FILE_NAME: &str = "Scarb.lock";
pub const DEFAULT_MODULE_MAIN_FILE: &str = "lib.cairo";
pub const DEFAULT_TESTS_PATH: &str = "tests";
pub const DEFAULT_TARGET_DIR_NAME: &str = "target";
pub const SCARB_IGNORE_FILE_NAME: &str = ".scarbignore";
pub static DEFAULT_SOURCE_PATH: LazyLock<Utf8PathBuf> =
    LazyLock::new(|| ["src", "lib.cairo"].iter().collect());
pub const DEFAULT_README_FILE_NAME: &str = "README.md";
pub const DEFAULT_LICENSE_FILE_NAME: &str = "LICENSE";
pub const STARKNET_PLUGIN_NAME: &str = "starknet";
pub const TEST_PLUGIN_NAME: &str = "cairo_test";
pub const TEST_ASSERTS_PLUGIN_NAME: &str = "assert_macros";
pub const CAIRO_RUN_PLUGIN_NAME: &str = "cairo_run";
pub const CARGO_MANIFEST_FILE_NAME: &str = "Cargo.toml";
pub const CARGO_LOCK_FILE_NAME: &str = "Cargo.lock";
