mod span_adapter;

use std::fmt::{Debug, Formatter};

use cairo_lang_defs::plugin::{PluginGeneratedFile, PluginResult};
use cairo_lang_filesystem::ids::CodeMapping;
use cairo_lang_filesystem::span::TextWidth;
use cairo_lang_macro::{AllocationContext, Diagnostic, TextSpan, TokenStream, TokenStreamMetadata};
use cairo_lang_syntax::attribute::structured::{AttributeArgVariant, AttributeStructurize};
use cairo_lang_syntax::node::ast::{Expr, PathSegment};
use cairo_lang_syntax::node::helpers::QueryAttrs;
use cairo_lang_syntax::node::{Terminal, TypedSyntaxNode, ast};
use itertools::Itertools;
use salsa::Database;

use crate::backend::{ExpansionId, ProcMacroBackend};
use crate::conversion::{CallSiteLocation, into_cairo_diagnostics};
use crate::expansion::{ExpansionKind, ExpansionQuery};
use crate::host::derive::span_adapter::DeriveAdapter;
use crate::host::{DERIVE_ATTR, ProcMacroHostPlugin, generate_code_mappings};
use crate::token_stream_builder::TokenStreamBuilder;

impl<B: ProcMacroBackend> ProcMacroHostPlugin<B> {
    /// Handle `#[derive(...)]` attribute.
    ///
    /// Returns a list of expansions that this plugin should apply.
    pub(crate) fn parse_derive<'db>(
        &self,
        db: &'db dyn Database,
        item_ast: ast::ModuleItem<'db>,
    ) -> Vec<DeriveFound<'db, B::Id>> {
        let attrs = match item_ast {
            ast::ModuleItem::Struct(struct_ast) => {
                Some(struct_ast.query_attr(db, DERIVE_ATTR).collect_vec())
            }
            ast::ModuleItem::Enum(enum_ast) => {
                Some(enum_ast.query_attr(db, DERIVE_ATTR).collect_vec())
            }
            _ => None,
        };

        attrs
            .unwrap_or_default()
            .iter()
            .map(|attr| attr.clone().structurize(db))
            .flat_map(|attr| attr.args.into_iter())
            .filter_map(|attr| {
                let AttributeArgVariant::Unnamed(value) = attr.clone().variant else {
                    return None;
                };
                let Expr::Path(path) = value else {
                    return None;
                };
                let path = path.segments(db);
                let path = path.elements(db);
                let path = path.last()?;
                let PathSegment::Simple(segment) = path else {
                    return None;
                };
                let ident = segment.ident(db);
                let value = ident.text(db).to_string(db);
                self.find_expansion(&ExpansionQuery::with_cairo_name(
                    value,
                    ExpansionKind::Derive,
                ))
                .map(|id| DeriveFound {
                    id,
                    call_site: CallSiteLocation::new(&segment, db),
                })
            })
            .collect_vec()
    }

    pub(crate) fn expand_derives<'db>(
        &self,
        db: &'db dyn Database,
        item_ast: ast::ModuleItem<'db>,
        derives: Vec<DeriveFound<'db, B::Id>>,
        stream_metadata: TokenStreamMetadata,
    ) -> Option<PluginResult<'db>> {
        let mut token_stream_builder = TokenStreamBuilder::new(db);
        token_stream_builder.add_node(item_ast.as_syntax_node());
        token_stream_builder.with_metadata(stream_metadata.clone());
        let mut aux_data = B::AuxData::default();
        let mut all_diagnostics: Vec<Diagnostic> = Vec::new();

        if derives.is_empty() {
            // No derives found - returning early.
            return None;
        }

        // We use call site of first derive found.
        let stable_ptr = derives[0].call_site.stable_ptr;

        let ctx = AllocationContext::default();
        let mut derived_code = String::new();
        let mut code_mappings = Vec::new();
        let mut current_width = TextWidth::default();

        let token_stream = token_stream_builder.build(&ctx);
        let (adapter, adapted_token_stream) = DeriveAdapter::adapt_token_stream(token_stream);

        for derive in derives.iter() {
            let call_site = adapter.adapted_call_site(&derive.call_site.span);
            let result = self.expand(
                db,
                &derive.id,
                call_site.clone(),
                TokenStream::empty(),
                adapted_token_stream.clone(),
                &mut aux_data,
            );

            // Register diagnostics.
            all_diagnostics.extend(adapter.adapt_diagnostics(result.diagnostics));

            if result.token_stream.is_empty() {
                // No code has been generated.
                // We do not need to do anything.
                continue;
            }

            code_mappings.extend(
                adapter.adapt_code_mappings(generate_code_mappings_with_offset(
                    &result.token_stream,
                    call_site,
                    current_width,
                )),
            );
            let ts_string = result.token_stream.to_string();
            current_width = current_width + TextWidth::from_str(&ts_string);
            derived_code.push_str(&ts_string);
        }

        Some(PluginResult {
            code: if derived_code.is_empty() {
                None
            } else {
                let msg = if derives.len() == 1 {
                    "the derive macro"
                } else {
                    "one of the derive macros"
                };
                let derive_names = derives
                    .iter()
                    .map(|derive| derive.id.expansion().cairo_name.to_string())
                    .join("`, `");
                let note = format!("this error originates in {msg}: `{derive_names}`");

                Some(PluginGeneratedFile {
                    name: "proc_macro_derive".into(),
                    code_mappings,
                    content: derived_code,
                    diagnostics_note: Some(note),
                    aux_data: self.backend().finish_aux_data(aux_data),
                    is_unhygienic: false,
                })
            },
            diagnostics: into_cairo_diagnostics(db, all_diagnostics, stable_ptr),
            // Note that we don't remove the original item here, unlike for attributes.
            // We do not add the original code to the generated file either.
            remove_original_item: false,
        })
    }
}

pub(crate) struct DeriveFound<'db, Id: ExpansionId> {
    id: Id,
    call_site: CallSiteLocation<'db>,
}

impl<'db, Id: ExpansionId> Debug for DeriveFound<'db, Id> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DeriveFound").field("id", &self.id).finish()
    }
}

pub(crate) fn generate_code_mappings_with_offset(
    token_stream: &TokenStream,
    call_site: TextSpan,
    offset: TextWidth,
) -> Vec<CodeMapping> {
    let mut mappings = generate_code_mappings(token_stream, call_site);
    for mapping in &mut mappings {
        mapping.span.start = mapping.span.start.add_width(offset);
        mapping.span.end = mapping.span.end.add_width(offset);
    }
    mappings
}
