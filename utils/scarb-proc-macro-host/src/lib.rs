//! The Cairo compiler plugin that drives procedural macro expansion.
//!
//! It is generic over a [`ProcMacroBackend`], implemented separately by Scarb and CairoLS.

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
