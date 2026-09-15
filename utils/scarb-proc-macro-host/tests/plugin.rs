//! End-to-end tests of the host plugin driven by a fake backend.
//!
//! The backend stands in for a real procedural macro: it declares a fixed set of expansions and
//! rewrites token streams in-memory. This exercises the whole host pipeline (finding what to
//! expand, building the input, adapting spans, mapping the output back) without loading any
//! dynamic library or talking to a proc macro server.

use std::sync::{Arc, Mutex};

use cairo_lang_defs::plugin::{
    DynGeneratedFileAuxData, GeneratedFileAuxData, MacroPlugin, MacroPluginMetadata, PluginResult,
};
use cairo_lang_filesystem::db::Edition;
use cairo_lang_filesystem::ids::CodeOrigin;
use cairo_lang_macro::{
    Diagnostic, ProcMacroResult, Severity, TextSpan, Token, TokenStream, TokenTree,
};
use cairo_lang_parser::utils::SimpleParserDatabase;
use cairo_lang_syntax::node::ast::{ModuleItem, SyntaxFile};
use cairo_lang_syntax::node::TypedSyntaxNode;
use cairo_lang_utils::ordered_hash_set::OrderedHashSet;
use cairo_lang_utils::unordered_hash_set::UnorderedHashSet;
use convert_case::{Case, Casing};
use salsa::Database;
use scarb_proc_macro_host::{
    Expansion, ExpansionId, ExpansionKind, ExpansionQuery, ProcMacroBackend, ProcMacroHostPlugin,
};

/// What a fake expansion does to the token stream it is given.
#[derive(Clone, Debug)]
enum Behaviour {
    /// Replace every occurrence of the first string with the second.
    Replace(&'static str, &'static str),
    /// Return the given code verbatim.
    Emit(&'static str),
    /// Return the input unchanged.
    Identity,
    /// Return nothing, which asks the compiler to remove the original item.
    Empty,
    /// Return the input unchanged, alongside an error diagnostic.
    Error(&'static str),
    /// Return the input unchanged, alongside auxiliary data.
    WithAuxData,
}

/// Stands in for whatever a backend collects alongside the generated code.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
struct FakeAuxData(Vec<String>);

#[typetag::serde]
impl GeneratedFileAuxData for FakeAuxData {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn eq(&self, other: &dyn GeneratedFileAuxData) -> bool {
        other
            .as_any()
            .downcast_ref::<Self>()
            .map(|other| self == other)
            .unwrap_or_default()
    }

    fn hash_value(&self) -> u64 {
        0
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct FakeId {
    expansion: Expansion,
    behaviour_index: usize,
}

impl ExpansionId for FakeId {
    fn expansion(&self) -> &Expansion {
        &self.expansion
    }
}

#[derive(Debug, Default)]
struct FakeBackend {
    expansions: Vec<(Expansion, Behaviour)>,
    executables: Vec<String>,
    /// Every expansion performed, in order, as `(expansion name, input)`.
    calls: Mutex<Vec<(String, String)>>,
    /// Every expansion the host reported back through `on_expanded`.
    observed: Mutex<Vec<String>>,
}

impl FakeBackend {
    fn new(expansions: Vec<(&str, ExpansionKind, Behaviour)>) -> Self {
        Self {
            expansions: expansions
                .into_iter()
                .map(|(name, kind, behaviour)| {
                    // Scarb exposes derives to Cairo code in upper camel case, and everything
                    // else under the name of the expansion function.
                    let cairo_name = if kind == ExpansionKind::Derive {
                        name.to_case(Case::UpperCamel)
                    } else {
                        name.to_string()
                    };
                    (
                        Expansion {
                            expansion_name: name.into(),
                            cairo_name: cairo_name.into(),
                            kind,
                        },
                        behaviour,
                    )
                })
                .collect(),
            executables: Vec::new(),
            calls: Default::default(),
            observed: Default::default(),
        }
    }

    fn with_executable(mut self, name: &str) -> Self {
        self.executables.push(name.to_string());
        self
    }

    fn calls(&self) -> Vec<(String, String)> {
        self.calls.lock().unwrap().clone()
    }

    fn observed(&self) -> Vec<String> {
        self.observed.lock().unwrap().clone()
    }

    fn names_of(&self, kind: ExpansionKind) -> Vec<String> {
        self.expansions
            .iter()
            .filter(|(expansion, _)| expansion.kind == kind)
            .map(|(expansion, _)| expansion.cairo_name.to_string())
            .collect()
    }
}

impl ProcMacroBackend for FakeBackend {
    type Id = FakeId;
    type AuxData = Vec<String>;

    fn find_expansion(&self, query: &ExpansionQuery) -> Option<FakeId> {
        self.expansions
            .iter()
            .position(|(expansion, _)| expansion.matches_query(query))
            .map(|behaviour_index| FakeId {
                expansion: self.expansions[behaviour_index].0.clone(),
                behaviour_index,
            })
    }

    fn inline_macros(&self) -> Vec<FakeId> {
        self.expansions
            .iter()
            .enumerate()
            .filter(|(_, (expansion, _))| expansion.kind == ExpansionKind::Inline)
            .map(|(behaviour_index, (expansion, _))| FakeId {
                expansion: expansion.clone(),
                behaviour_index,
            })
            .collect()
    }

    fn declared_attributes(&self) -> Vec<String> {
        let mut names = self.names_of(ExpansionKind::Attr);
        names.extend(self.executables.clone());
        names
    }

    fn executable_attributes(&self) -> Vec<String> {
        self.executables.clone()
    }

    fn declared_derives(&self) -> Vec<String> {
        self.names_of(ExpansionKind::Derive)
    }

    fn expand(
        &self,
        _db: &dyn Database,
        id: &FakeId,
        call_site: TextSpan,
        _args: TokenStream,
        item: TokenStream,
    ) -> ProcMacroResult {
        let input = item.to_string();
        self.calls
            .lock()
            .unwrap()
            .push((id.expansion.expansion_name.to_string(), input.clone()));

        let (token_stream, diagnostics) = match &self.expansions[id.behaviour_index].1 {
            Behaviour::Identity => (item, Vec::new()),
            Behaviour::Empty => (TokenStream::empty(), Vec::new()),
            Behaviour::Replace(from, to) => {
                let content = input.replace(from, to);
                (single_token(content, whole_span(&item, &call_site)), Vec::new())
            }
            Behaviour::Emit(code) => (
                single_token(code.to_string(), call_site.clone()),
                Vec::new(),
            ),
            Behaviour::Error(message) => (
                item,
                vec![Diagnostic::spanned(
                    call_site.clone(),
                    Severity::Error,
                    message.to_string(),
                )],
            ),
            Behaviour::WithAuxData => (item, Vec::new()),
        };

        let aux_data = matches!(
            self.expansions[id.behaviour_index].1,
            Behaviour::WithAuxData
        )
        .then(|| cairo_lang_macro::AuxData::new(b"aux".to_vec()));

        ProcMacroResult {
            token_stream,
            aux_data,
            diagnostics,
            full_path_markers: Vec::new(),
        }
    }

    fn on_expanded(&self, id: &FakeId, _result: &ProcMacroResult, aux_data: &mut Vec<String>) {
        self.observed
            .lock()
            .unwrap()
            .push(id.expansion.expansion_name.to_string());
        aux_data.push(id.expansion.expansion_name.to_string());
    }

    fn finish_aux_data(&self, aux_data: Vec<String>) -> Option<DynGeneratedFileAuxData> {
        (!aux_data.is_empty()).then(|| DynGeneratedFileAuxData::new(FakeAuxData(aux_data)))
    }
}

fn single_token(content: String, span: TextSpan) -> TokenStream {
    if content.is_empty() {
        return TokenStream::empty();
    }
    TokenStream::new(vec![TokenTree::Ident(Token::new(content, span))])
}

/// The span covering a whole token stream, falling back to the call site when it carries no tokens.
fn whole_span(token_stream: &TokenStream, fallback: &TextSpan) -> TextSpan {
    let (Some(TokenTree::Ident(first)), Some(TokenTree::Ident(last))) =
        (token_stream.tokens.first(), token_stream.tokens.last())
    else {
        return fallback.clone();
    };
    TextSpan::new(first.span.start, last.span.end)
}

/// Runs the host plugin over every item of `code`, returning one result per item.
fn expand_all(backend: FakeBackend, code: &str) -> (Arc<FakeBackend>, Vec<ExpandedItem>) {
    let db = SimpleParserDatabase::default();
    let backend = Arc::new(backend);
    let plugin = ProcMacroHostPlugin::new(backend.clone());

    let parsed = db.parse_virtual(code).expect("test input should parse");
    let syntax_file = SyntaxFile::from_syntax_node(&db, parsed);

    let cfg_set = Default::default();
    let declared_derives = OrderedHashSet::default();
    let allowed_features = OrderedHashSet::default();
    let metadata = MacroPluginMetadata {
        cfg_set: &cfg_set,
        declared_derives: &declared_derives,
        allowed_features: &allowed_features,
        edition: Edition::V2024_07,
    };

    let results = syntax_file
        .items(&db)
        .elements(&db)
        .map(|item: ModuleItem<'_>| {
            let result = plugin.generate_code(&db, item, &metadata);
            ExpandedItem::new(result)
        })
        .collect();

    (backend, results)
}

/// The parts of a [`PluginResult`] the tests care about, detached from the database lifetime.
struct ExpandedItem {
    content: Option<String>,
    origins: Vec<CodeOrigin>,
    diagnostics: Vec<String>,
    remove_original_item: bool,
    aux_data: Option<Vec<String>>,
}

impl ExpandedItem {
    fn new(result: PluginResult<'_>) -> Self {
        Self {
            content: result.code.as_ref().map(|code| code.content.clone()),
            origins: result
                .code
                .as_ref()
                .map(|code| code.code_mappings.iter().map(|m| m.origin.clone()).collect())
                .unwrap_or_default(),
            diagnostics: result
                .diagnostics
                .iter()
                .map(|d| d.message.clone())
                .collect(),
            remove_original_item: result.remove_original_item,
            aux_data: result.code.as_ref().and_then(|code| {
                let aux_data = code.aux_data.as_ref()?;
                Some(aux_data.as_any().downcast_ref::<FakeAuxData>()?.0.clone())
            }),
        }
    }
}

#[test]
fn attribute_is_expanded_and_removed_from_the_item() {
    let (backend, results) = expand_all(
        FakeBackend::new(vec![(
            "rename",
            ExpansionKind::Attr,
            Behaviour::Replace("old", "new"),
        )]),
        "#[rename]\nfn old() {}\n",
    );

    let item = results.into_iter().next().unwrap();
    assert_eq!(item.content.as_deref(), Some("fn new() {}\n"));
    assert!(item.remove_original_item);
    assert!(item.diagnostics.is_empty());

    // The expandable attribute is not part of the input handed to the macro.
    let calls = backend.calls();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].0, "rename");
    assert_eq!(calls[0].1, "fn old() {}\n");
}

#[test]
fn attribute_returning_nothing_removes_the_item() {
    let (_, results) = expand_all(
        FakeBackend::new(vec![("strip", ExpansionKind::Attr, Behaviour::Empty)]),
        "#[strip]\nfn gone() {}\n",
    );

    let item = results.into_iter().next().unwrap();
    assert_eq!(item.content, None);
    assert!(item.remove_original_item);
}

#[test]
fn unchanged_attribute_expansion_leaves_the_item_alone() {
    let (_, results) = expand_all(
        FakeBackend::new(vec![("keep", ExpansionKind::Attr, Behaviour::Identity)]),
        "#[keep]\nfn same() {}\n",
    );

    // The only attribute expanded produced no change, so there is nothing to rewrite.
    let item = results.into_iter().next().unwrap();
    assert_eq!(item.content, None);
    assert!(!item.remove_original_item);
}

#[test]
fn diagnostics_are_reported_against_the_call_site() {
    let (_, results) = expand_all(
        FakeBackend::new(vec![(
            "failing",
            ExpansionKind::Attr,
            Behaviour::Error("something went wrong"),
        )]),
        "#[failing]\nfn foo() {}\n",
    );

    let item = results.into_iter().next().unwrap();
    assert_eq!(item.diagnostics, vec!["something went wrong".to_string()]);
}

#[test]
fn derives_are_expanded_one_by_one_in_source_order() {
    let (backend, results) = expand_all(
        FakeBackend::new(vec![
            (
                "first",
                ExpansionKind::Derive,
                Behaviour::Emit("impl First {}"),
            ),
            (
                "second",
                ExpansionKind::Derive,
                Behaviour::Emit("impl Second {}"),
            ),
        ]),
        "#[derive(First, Second)]\nstruct S {}\n",
    );

    let item = results.into_iter().next().unwrap();
    assert_eq!(
        item.content.as_deref(),
        Some("impl First {}impl Second {}")
    );
    // Derives do not replace the item they are applied to.
    assert!(!item.remove_original_item);

    let calls = backend.calls();
    assert_eq!(
        calls.iter().map(|(name, _)| name.as_str()).collect::<Vec<_>>(),
        vec!["first", "second"]
    );
    // Every derive sees the whole original item.
    assert!(calls[0].1.contains("struct S {}"));
    assert_eq!(calls[0].1, calls[1].1);
}

#[test]
fn derives_we_do_not_declare_are_ignored() {
    let (_, results) = expand_all(
        FakeBackend::new(vec![(
            "Unknown",
            ExpansionKind::Derive,
            Behaviour::Emit("impl Unknown {}"),
        )]),
        "#[derive(Drop)]\nstruct S {}\n",
    );

    // `Drop` is not ours, so nothing is generated.
    let item = results.into_iter().next().unwrap();
    assert_eq!(item.content, None);
}

#[test]
fn module_level_inline_macro_replaces_the_item() {
    let (_, results) = expand_all(
        FakeBackend::new(vec![(
            "generate",
            ExpansionKind::Inline,
            Behaviour::Emit("fn generated() {}"),
        )]),
        "generate!();\n",
    );

    let item = results.into_iter().next().unwrap();
    assert_eq!(item.content.as_deref(), Some("fn generated() {}"));
    assert!(item.remove_original_item);
}

#[test]
fn inner_attributes_of_impl_functions_are_expanded() {
    let (backend, results) = expand_all(
        FakeBackend::new(vec![(
            "inner",
            ExpansionKind::Attr,
            Behaviour::Replace("old", "new"),
        )]),
        "impl A of B {\n    #[inner]\n    fn old() {}\n}\n",
    );

    let item = results.into_iter().next().unwrap();
    let content = item.content.expect("impl body should be rewritten");
    assert!(content.contains("fn new() {}"), "got: {content}");
    assert!(item.remove_original_item);

    let calls = backend.calls();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].0, "inner");
}

#[test]
fn items_without_our_macros_are_not_touched() {
    let (backend, results) = expand_all(
        FakeBackend::new(vec![(
            "unused",
            ExpansionKind::Attr,
            Behaviour::Replace("a", "b"),
        )]),
        "fn plain() {}\n",
    );

    let item = results.into_iter().next().unwrap();
    assert_eq!(item.content, None);
    assert!(!item.remove_original_item);
    assert!(backend.calls().is_empty());
}

#[test]
fn executable_attributes_are_declared_but_never_expanded() {
    let backend = FakeBackend::new(vec![]).with_executable("marker");
    let db = SimpleParserDatabase::default();
    let backend = Arc::new(backend);
    let plugin = ProcMacroHostPlugin::new(backend.clone());

    let declared: UnorderedHashSet<_> = plugin
        .executable_attributes(&db)
        .into_iter()
        .map(|s| s.to_string(&db))
        .collect();
    assert!(declared.contains(&"marker".to_string()));

    let (backend, results) = expand_all(
        FakeBackend::new(vec![]).with_executable("marker"),
        "#[marker]\nfn foo() {}\n",
    );
    let item = results.into_iter().next().unwrap();
    assert_eq!(item.content, None);
    assert!(backend.calls().is_empty());
}

#[test]
fn generated_code_is_mapped_back_onto_the_original_source() {
    let (_, results) = expand_all(
        FakeBackend::new(vec![(
            "rename",
            ExpansionKind::Attr,
            Behaviour::Replace("old", "new"),
        )]),
        "#[rename]\nfn old() {}\n",
    );

    let item = results.into_iter().next().unwrap();
    // Every piece of generated code points back at a span of the original file, so that the IDE
    // can navigate from expanded code to the code the user wrote.
    assert!(!item.origins.is_empty());
    assert!(
        item.origins
            .iter()
            .all(|origin| matches!(origin, CodeOrigin::Span(_) | CodeOrigin::CallSite(_)))
    );
}

#[test]
fn ast_item_kinds_without_attribute_support_are_ignored() {
    let (backend, results) = expand_all(
        FakeBackend::new(vec![(
            "rename",
            ExpansionKind::Attr,
            Behaviour::Replace("old", "new"),
        )]),
        "use core::old;\n",
    );

    // `use` items do support attributes, but carry none of ours here.
    let item = results.into_iter().next().unwrap();
    assert_eq!(item.content, None);
    assert!(backend.calls().is_empty());
}

#[test]
fn only_the_first_attribute_is_expanded_per_pass() {
    // Attributes are expanded one per `generate_code` call. Even when an expansion changes
    // nothing, the item must still be rewritten so the compiler calls us again for the next
    // attribute, which would otherwise never run.
    let (backend, results) = expand_all(
        FakeBackend::new(vec![
            ("first", ExpansionKind::Attr, Behaviour::Identity),
            ("second", ExpansionKind::Attr, Behaviour::Identity),
        ]),
        "#[first]\n#[second]\nfn foo() {}\n",
    );

    let item = results.into_iter().next().unwrap();
    let content = item.content.expect("the item must be rewritten");
    assert!(content.contains("#[second]"), "got: {content}");
    assert!(!content.contains("#[first]"), "got: {content}");
    assert!(item.remove_original_item);

    assert_eq!(
        backend
            .calls()
            .iter()
            .map(|(name, _)| name.as_str())
            .collect::<Vec<_>>(),
        vec!["first"]
    );
}

#[test]
fn an_unchanged_attribute_still_rewrites_when_derives_are_pending() {
    // The "nothing changed, leave the item alone" shortcut only applies when there is no further
    // work for this item. A pending derive is further work.
    let (_, results) = expand_all(
        FakeBackend::new(vec![
            ("keep", ExpansionKind::Attr, Behaviour::Identity),
            (
                "first",
                ExpansionKind::Derive,
                Behaviour::Emit("impl First {}"),
            ),
        ]),
        "#[keep]\n#[derive(First)]\nstruct S {}\n",
    );

    let item = results.into_iter().next().unwrap();
    assert!(item.content.is_some());
}

#[test]
fn inner_attributes_of_trait_functions_are_expanded() {
    // The trait branch rebuilds the item separately from the impl branch, so it needs its own
    // coverage.
    let (backend, results) = expand_all(
        FakeBackend::new(vec![(
            "inner",
            ExpansionKind::Attr,
            Behaviour::Replace("old", "new"),
        )]),
        "trait T {\n    #[inner]\n    fn old();\n}\n",
    );

    let item = results.into_iter().next().unwrap();
    let content = item.content.expect("trait body should be rewritten");
    assert!(content.contains("fn new();"), "got: {content}");
    assert!(item.remove_original_item);
    assert_eq!(backend.calls().len(), 1);
}

#[test]
fn executable_attributes_are_left_on_the_expanded_item() {
    // An executable attribute is consumed later in the build, so it must survive expansion of the
    // attribute sitting next to it, and be visible to the macro that runs.
    let (backend, results) = expand_all(
        FakeBackend::new(vec![(
            "rename",
            ExpansionKind::Attr,
            Behaviour::Replace("old", "new"),
        )])
        .with_executable("marker"),
        "#[marker]\n#[rename]\nfn old() {}\n",
    );

    let item = results.into_iter().next().unwrap();
    let content = item.content.expect("the item should be rewritten");
    assert!(content.contains("#[marker]"), "got: {content}");
    assert!(content.contains("fn new() {}"), "got: {content}");

    let calls = backend.calls();
    assert_eq!(calls.len(), 1);
    assert!(calls[0].1.contains("#[marker]"), "got: {}", calls[0].1);
}

#[test]
fn aux_data_from_every_expansion_is_attached_to_the_generated_file() {
    let (_, results) = expand_all(
        FakeBackend::new(vec![
            (
                "first",
                ExpansionKind::Derive,
                Behaviour::Emit("impl First {}"),
            ),
            (
                "second",
                ExpansionKind::Derive,
                Behaviour::Emit("impl Second {}"),
            ),
        ]),
        "#[derive(First, Second)]\nstruct S {}\n",
    );

    let item = results.into_iter().next().unwrap();
    assert_eq!(
        item.aux_data,
        Some(vec!["first".to_string(), "second".to_string()])
    );
}

#[test]
fn expansions_are_reported_even_when_they_produce_no_code() {
    // Backends rely on being told about every expansion, not just the ones that generated code,
    // because macros can emit side output without changing the item.
    let (backend, _) = expand_all(
        FakeBackend::new(vec![("strip", ExpansionKind::Attr, Behaviour::Empty)]),
        "#[strip]\nfn gone() {}\n",
    );

    assert_eq!(backend.observed(), vec!["strip".to_string()]);
}

#[test]
fn a_macro_emitting_aux_data_always_rewrites_the_item() {
    // The "nothing changed" shortcut must not fire when the macro emitted side output, or that
    // output would be dropped along with the generated file.
    let (_, results) = expand_all(
        FakeBackend::new(vec![("collect", ExpansionKind::Attr, Behaviour::WithAuxData)]),
        "#[collect]\nfn foo() {}\n",
    );

    let item = results.into_iter().next().unwrap();
    assert!(item.content.is_some(), "aux data must survive");
    assert_eq!(item.aux_data, Some(vec!["collect".to_string()]));
}

#[test]
fn derives_are_matched_on_enums_too() {
    let (_, results) = expand_all(
        FakeBackend::new(vec![(
            "first",
            ExpansionKind::Derive,
            Behaviour::Emit("impl First {}"),
        )]),
        "#[derive(First)]\nenum E { A }\n",
    );

    let item = results.into_iter().next().unwrap();
    assert_eq!(item.content.as_deref(), Some("impl First {}"));
}

#[test]
fn a_derive_producing_no_code_is_skipped() {
    let (backend, results) = expand_all(
        FakeBackend::new(vec![
            ("first", ExpansionKind::Derive, Behaviour::Empty),
            (
                "second",
                ExpansionKind::Derive,
                Behaviour::Emit("impl Second {}"),
            ),
        ]),
        "#[derive(First, Second)]\nstruct S {}\n",
    );

    let item = results.into_iter().next().unwrap();
    assert_eq!(item.content.as_deref(), Some("impl Second {}"));
    // Both still ran, so both can report diagnostics and side output.
    assert_eq!(backend.observed().len(), 2);
}

#[test]
fn module_level_inline_macros_with_a_qualified_path_are_left_alone() {
    // A path with more than one segment is not something this host claims.
    let (backend, results) = expand_all(
        FakeBackend::new(vec![(
            "generate",
            ExpansionKind::Inline,
            Behaviour::Emit("fn generated() {}"),
        )]),
        "other::generate!();\n",
    );

    let item = results.into_iter().next().unwrap();
    assert_eq!(item.content, None);
    assert!(backend.calls().is_empty());
}

#[test]
fn a_whole_input_span_maps_back_onto_the_whole_item() {
    // A macro that rebuilds the item from its string form emits one token spanning the entire
    // input it was given. That span straddles the code before the expandable attribute and the
    // code after it, so both of its ends have to move by their own amount. Mapping the end with
    // the region of the start used to cut the mapping short by the width of the attribute, which
    // sent goto and diagnostics inside the generated code to the wrong place.
    let source = "#[marker]\n#[rename]\nfn old() {}\n";
    let (_, results) = expand_all(
        FakeBackend::new(vec![(
            "rename",
            ExpansionKind::Attr,
            Behaviour::Replace("old", "new"),
        )])
        .with_executable("marker"),
        source,
    );

    let item = results.into_iter().next().unwrap();
    let spans: Vec<_> = item
        .origins
        .iter()
        .filter_map(|origin| match origin {
            CodeOrigin::Span(span) => Some((span.start.as_u32(), span.end.as_u32())),
            _ => None,
        })
        .collect();

    // The item starts at offset 0 and runs to the end of the file.
    let item_end = source.len() as u32;
    assert!(
        spans.contains(&(0, item_end)),
        "expected a mapping covering the whole item (0..{item_end}), got {spans:?}"
    );
}
