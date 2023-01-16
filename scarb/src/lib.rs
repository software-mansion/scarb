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

pub const SCARB_ENV: &str = "SCARB";
pub const CORELIB_REPO_URL: &str = "https://github.com/starkware-libs/cairo.git";
