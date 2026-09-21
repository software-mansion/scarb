//! The Cairo compiler plugin that drives procedural macro expansion.
//!
//! This crate contains the logic that sits between the Cairo compiler and a procedural macro
//! implementation: finding which macros apply to an AST item, building the input token stream,
//! adapting token spans so the macro sees consecutive input, and mapping the expansion output
//! back onto the original source code.
//!
//! It is generic over a [`ProcMacroBackend`], which supplies the list of available macro
//! expansions and performs the actual expansion. Scarb implements that backend by calling into
//! procedural macro dynamic libraries loaded in-process. CairoLS implements it by talking to
//! `scarb proc-macro-server`.

mod backend;
mod conversion;
mod expansion;
mod host;
mod span_utils;
mod syntax_ext;
mod token_stream_builder;

pub use backend::{ExpansionId, FULL_PATH_MARKER_KEY, ProcMacroBackend};
pub use expansion::{Expansion, ExpansionKind, ExpansionQuery};
pub use host::{ProcMacroHostPlugin, ProcMacroInlinePlugin};
