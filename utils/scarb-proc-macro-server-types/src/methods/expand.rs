use crate::scope::ProcMacroScope;

use super::Method;
use super::ProcMacroResult;
use cairo_lang_macro::{TextSpan, TokenStream};
use serde::{Deserialize, Serialize};

/// Parameters for expanding a specific attribute macro.
///
/// This structure is used to specify which attribute macro should be expanded
/// based on the provided item and arguments.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, Hash)]
pub struct ExpandAttributeParams {
    /// The project scope in which the action is requested.
    pub context: ProcMacroScope,
    /// The name of the attribute macro to be expanded.
    pub attr: String,
    /// The token stream representing arguments passed to the macro.
    pub args: TokenStream,
    /// The token stream representing the item on which the macro is applied.
    pub item: TokenStream,
    /// The span of the "adapted" call site
    pub adapted_call_site: TextSpan,
}

/// Represents a request to expand a single attribute macro.
pub struct ExpandAttribute;

impl Method for ExpandAttribute {
    const METHOD: &'static str = "expandAttribute";

    type Params = ExpandAttributeParams;
    type Response = ProcMacroResult;
}

/// A single derive macro applied to an item.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, Hash)]
pub struct Derive {
    /// Name of derive macro.
    pub name: String,
    /// The span of the macro call code.
    pub call_site: TextSpan,
}

/// Parameters for expanding all derive macros of a single item.
///
/// These parameters specify the derive macros to be expanded and the item they apply to.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, Hash)]
pub struct ExpandDeriveParams {
    /// The project scope in which the action is requested.
    pub context: ProcMacroScope,
    /// The derive macros to be expanded, each with its own call site.
    pub derives: Vec<Derive>,
    /// The token stream of the item to which the derive macros are applied.
    pub item: TokenStream,
}

/// Represents a request to expand the derive macros of one item.
pub struct ExpandDerive;

impl Method for ExpandDerive {
    const METHOD: &'static str = "expandDerive";

    type Params = ExpandDeriveParams;
    /// One result per requested derive, in the same order.
    type Response = Vec<ProcMacroResult>;
}

/// Parameters for expanding a single inline macro.
///
/// Specifies the inline macro to expand along with its arguments.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, Hash)]
pub struct ExpandInlineMacroParams {
    /// The project scope in which the action is requested.
    pub context: ProcMacroScope,
    /// The macro_name! of the inline macro to be expanded.
    pub name: String,
    /// The token stream representing arguments passed to the macro.
    pub args: TokenStream,
    /// The span of the macro call code
    pub call_site: TextSpan,
}

/// Represents a request to expand a single inline macro.
pub struct ExpandInline;

impl Method for ExpandInline {
    const METHOD: &'static str = "expandInline";

    type Params = ExpandInlineMacroParams;
    type Response = ProcMacroResult;
}
