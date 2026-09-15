use std::fmt::Debug;

use cairo_lang_defs::plugin::DynGeneratedFileAuxData;
use cairo_lang_macro::{ProcMacroResult, TextSpan, TokenStream};
use salsa::Database;

use crate::expansion::{Expansion, ExpansionQuery};

/// Identifies a single expansion of a single procedural macro, as understood by a
/// [`ProcMacroBackend`].
///
/// The host only ever needs the [`Expansion`] metadata out of it: the name the macro is called by
/// in Cairo code, the name of the expansion function in the macro implementation, and the kind of
/// the expansion. Everything else an implementation needs to locate the macro (a package id, a
/// fingerprint, a connection scope) is opaque to the host and carried along untouched.
pub trait ExpansionId: Clone + Debug + Send + Sync + 'static {
    fn expansion(&self) -> &Expansion;
}

/// Provides procedural macro expansions to the [`crate::ProcMacroHostPlugin`].
///
/// The host plugin implements the Cairo side of procedural macro expansion: it decides which
/// macros apply to an AST item, builds the input token streams, adapts token spans, and maps the
/// expansion output back onto the original source code. It delegates the two things it cannot
/// know by itself to this trait: which macros exist, and what a given macro does to a token
/// stream.
///
/// Scarb implements this by calling into procedural macro dynamic libraries loaded in-process.
/// CairoLS implements it by requesting expansions from `scarb proc-macro-server` and reading the
/// results out of its cache.
pub trait ProcMacroBackend: Debug + Send + Sync + 'static {
    /// See [`ExpansionId`].
    type Id: ExpansionId;

    /// Side output collected while generating a single file, which the host passes through
    /// without interpreting.
    ///
    /// Scarb uses this to collect auxiliary data emitted by macros. Backends that have no use for
    /// it should set it to `()`.
    type AuxData: Default;

    /// Finds an expansion matching the query, if this backend provides one.
    fn find_expansion(&self, query: &ExpansionQuery) -> Option<Self::Id>;

    /// All inline macro expansions provided by this backend.
    ///
    /// Unlike attributes and derives, inline macros are registered with the compiler as separate
    /// plugins, one per expansion, so the host needs the ids rather than just the names.
    fn inline_macros(&self) -> Vec<Self::Id>;

    /// Names of all attributes this backend handles, including executable attributes.
    fn declared_attributes(&self) -> Vec<String>;

    /// Names of attributes that only mark code for later processing and are never expanded.
    fn executable_attributes(&self) -> Vec<String>;

    /// Names of all derives this backend handles, as written in Cairo code.
    fn declared_derives(&self) -> Vec<String>;

    /// Expands a single macro.
    ///
    /// `call_site`, `args` and `item` spans have already been adapted by the host, and the
    /// expansion output is expected to use the same coordinate space.
    fn expand(
        &self,
        db: &dyn Database,
        id: &Self::Id,
        call_site: TextSpan,
        args: TokenStream,
        item: TokenStream,
    ) -> ProcMacroResult;

    /// Called immediately after every [`Self::expand`], including for expansions that produced no
    /// code.
    ///
    /// This is where a backend collects whatever the host does not interpret, such as auxiliary
    /// data and full path markers.
    fn on_expanded(
        &self,
        _id: &Self::Id,
        _result: &ProcMacroResult,
        _aux_data: &mut Self::AuxData,
    ) {
    }

    /// Turns the side output collected for one generated file into auxiliary data attached to it.
    fn finish_aux_data(&self, _aux_data: Self::AuxData) -> Option<DynGeneratedFileAuxData> {
        None
    }

    /// Documentation of an expansion, shown by IDEs.
    fn doc(&self, _id: &Self::Id) -> Option<String> {
        None
    }
}
