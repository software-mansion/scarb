use std::fmt::Debug;

use cairo_lang_defs::plugin::DynGeneratedFileAuxData;
use cairo_lang_macro::{ProcMacroResult, TextSpan, TokenStream};
use salsa::Database;

use crate::expansion::{Expansion, ExpansionKind, ExpansionQuery};

/// Attribute that macros put on generated items to get their full path resolved.
pub const FULL_PATH_MARKER_KEY: &str = "proc_macro::full_path_marker";

/// Identifies a single macro expansion of a [`ProcMacroBackend`].
pub trait ExpansionId: Clone + Debug + Send + Sync + 'static {
    fn expansion(&self) -> &Expansion;
}

/// Lists the available macros and expands them for the [`crate::ProcMacroHostPlugin`].
pub trait ProcMacroBackend: Debug + Send + Sync + 'static {
    type Id: ExpansionId;

    /// Side output collected while generating a single file.
    type AuxData: Default;

    /// Every expansion this backend provides, including executable attributes.
    fn expansions(&self) -> &[Self::Id];

    /// Finds an expansion matching the query, if this backend provides one.
    fn find_expansion(&self, query: &ExpansionQuery) -> Option<Self::Id> {
        self.expansions()
            .iter()
            .find(|id| id.expansion().matches_query(query))
            .cloned()
    }

    /// All inline macro expansions provided by this backend.
    fn inline_macros(&self) -> Vec<Self::Id> {
        self.expansions()
            .iter()
            .filter(|id| id.expansion().kind == ExpansionKind::Inline)
            .cloned()
            .collect()
    }

    /// Names of all attributes this backend handles.
    fn declared_attributes(&self) -> Vec<String> {
        let mut names = cairo_names_of(self, &[ExpansionKind::Attr, ExpansionKind::Executable]);
        names.push(FULL_PATH_MARKER_KEY.to_string());
        names
    }

    /// Names of executable attributes, which are never expanded.
    fn executable_attributes(&self) -> Vec<String> {
        cairo_names_of(self, &[ExpansionKind::Executable])
    }

    /// Names of all derives this backend handles, as written in Cairo code.
    fn declared_derives(&self) -> Vec<String> {
        cairo_names_of(self, &[ExpansionKind::Derive])
    }

    /// Expands a single macro.
    fn expand(
        &self,
        db: &dyn Database,
        id: &Self::Id,
        call_site: TextSpan,
        args: TokenStream,
        item: TokenStream,
    ) -> ProcMacroResult;

    /// Expands all derives applied to one item, one result per derive, in the given order.
    ///
    /// Backends talking to another process override this to expand an item in a single request.
    fn expand_derives(
        &self,
        db: &dyn Database,
        derives: &[(Self::Id, TextSpan)],
        item: TokenStream,
    ) -> Vec<ProcMacroResult> {
        derives
            .iter()
            .map(|(id, call_site)| {
                self.expand(
                    db,
                    id,
                    call_site.clone(),
                    TokenStream::empty(),
                    item.clone(),
                )
            })
            .collect()
    }

    /// Called after every [`Self::expand`].
    fn on_expanded(
        &self,
        _id: &Self::Id,
        _result: &ProcMacroResult,
        _aux_data: &mut Self::AuxData,
    ) {
    }

    /// Converts the collected side output into auxiliary data of the generated file.
    fn finish_aux_data(&self, _aux_data: Self::AuxData) -> Option<DynGeneratedFileAuxData> {
        None
    }

    /// Documentation of an expansion, shown by IDEs.
    fn doc(&self, _id: &Self::Id) -> Option<String> {
        None
    }
}

/// Cairo names of the backend's expansions of the given kinds.
fn cairo_names_of<B: ProcMacroBackend + ?Sized>(
    backend: &B,
    kinds: &[ExpansionKind],
) -> Vec<String> {
    backend
        .expansions()
        .iter()
        .map(ExpansionId::expansion)
        .filter(|expansion| kinds.contains(&expansion.kind))
        .map(|expansion| expansion.cairo_name.to_string())
        .collect()
}
