use crate::compiler::plugin::proc_macro::ExpansionQuery;
use crate::compiler::plugin::proc_macro::expansion::ExpansionKind;
use crate::compiler::plugin::proc_macro::v2::host::aux_data::{EmittedAuxData, ProcMacroAuxData};
use crate::compiler::plugin::proc_macro::v2::host::conversion::{
    CallSiteLocation, into_cairo_diagnostics,
};
use crate::compiler::plugin::proc_macro::v2::host::{DERIVE_ATTR, generate_code_mappings};
use crate::compiler::plugin::proc_macro::v2::{
    ProcMacroHostPlugin, ProcMacroId, TokenStreamBuilder,
};
use cairo_lang_defs::plugin::{DynGeneratedFileAuxData, PluginGeneratedFile, PluginResult};
use cairo_lang_filesystem::ids::CodeMapping;
use cairo_lang_filesystem::span::TextWidth;
use cairo_lang_macro::{AllocationContext, Diagnostic, TextSpan, TokenStream, TokenStreamMetadata};
use cairo_lang_syntax::attribute::structured::{AttributeArgVariant, AttributeStructurize};
use cairo_lang_syntax::node::ast::{Expr, PathSegment};
use cairo_lang_syntax::node::db::SyntaxGroup;
use cairo_lang_syntax::node::helpers::QueryAttrs;
use cairo_lang_syntax::node::{Terminal, TypedSyntaxNode, ast};
use itertools::Itertools;
use std::fmt::{Debug, Formatter};

impl ProcMacroHostPlugin {
    /// Handle `#[derive(...)]` attribute.
    ///
    /// Returns a list of expansions that this plugin should apply.
    fn parse_derive(&self, db: &dyn SyntaxGroup, item_ast: ast::ModuleItem) -> Vec<DeriveFound> {
        let attrs = match item_ast {
            ast::ModuleItem::Struct(struct_ast) => Some(struct_ast.query_attr(db, DERIVE_ATTR)),
            ast::ModuleItem::Enum(enum_ast) => Some(enum_ast.query_attr(db, DERIVE_ATTR)),
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
                let value = ident.text(db).to_string();
                self.find_expansion(&ExpansionQuery::with_cairo_name(
                    value,
                    ExpansionKind::Derive,
                ))
                .map(|id| DeriveFound {
                    id,
                    call_site: CallSiteLocation::new(segment, db),
                })
            })
            .collect_vec()
    }

    pub fn expand_derives(
        &self,
        db: &dyn SyntaxGroup,
        item_ast: ast::ModuleItem,
        stream_metadata: TokenStreamMetadata,
    ) -> Option<PluginResult> {
        let mut token_stream_builder = TokenStreamBuilder::new(db);
        token_stream_builder.add_node(item_ast.as_syntax_node());
        token_stream_builder.with_metadata(stream_metadata.clone());
        let mut aux_data = EmittedAuxData::default();
        let mut all_diagnostics: Vec<Diagnostic> = Vec::new();

        // All derives to be applied.
        let derives = self.parse_derive(db, item_ast.clone());

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

        for derive in derives.iter() {
            let call_site = &derive.call_site;
            let derive = &derive.id;
            let token_stream = token_stream_builder.build(&ctx);
            let result = self
                .instance(derive.package_id)
                .try_v2()
                .expect("procedural macro using v1 api used in a context expecting v2 api")
                .generate_code(
                    derive.expansion.expansion_name.clone(),
                    call_site.span.clone(),
                    TokenStream::empty(),
                    token_stream,
                );

            // Register diagnostics.
            all_diagnostics.extend(result.diagnostics);

            // Register aux data.
            if let Some(new_aux_data) = result.aux_data {
                aux_data.push(ProcMacroAuxData::new(
                    new_aux_data.into(),
                    ProcMacroId::new(derive.package_id, derive.expansion.clone()),
                ));
            }

            if result.token_stream.is_empty() {
                // No code has been generated.
                // We do not need to do anything.
                continue;
            }

            code_mappings.extend(generate_code_mappings_with_offset(
                &result.token_stream,
                call_site.span.clone(),
                current_width,
            ));
            current_width = current_width + TextWidth::from_str(&result.token_stream.to_string());
            derived_code.push_str(&result.token_stream.to_string());
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
                    .map(|derive| derive.id.expansion.cairo_name.to_string())
                    .join("`, `");
                let note = format!("this error originates in {msg}: `{derive_names}`");

                Some(PluginGeneratedFile {
                    name: "proc_macro_derive".into(),
                    code_mappings,
                    content: derived_code,
                    diagnostics_note: Some(note),
                    aux_data: if aux_data.is_empty() {
                        None
                    } else {
                        Some(DynGeneratedFileAuxData::new(aux_data))
                    },
                })
            },
            diagnostics: into_cairo_diagnostics(db, all_diagnostics, stable_ptr),
            // Note that we don't remove the original item here, unlike for attributes.
            // We do not add the original code to the generated file either.
            remove_original_item: false,
        })
    }
}

struct DeriveFound {
    id: ProcMacroId,
    call_site: CallSiteLocation,
}

impl Debug for DeriveFound {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DeriveFound").field("id", &self.id).finish()
    }
}

pub fn generate_code_mappings_with_offset(
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
