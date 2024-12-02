use crate::compiler::plugin::proc_macro::host::aux_data::{EmittedAuxData, ProcMacroAuxData};
use crate::compiler::plugin::proc_macro::host::{
    generate_code_mappings, into_cairo_diagnostics, DERIVE_ATTR,
};
use crate::compiler::plugin::proc_macro::{
    Expansion, ExpansionKind, ProcMacroHostPlugin, ProcMacroId, TokenStreamBuilder,
};
use cairo_lang_defs::patcher::PatchBuilder;
use cairo_lang_defs::plugin::{DynGeneratedFileAuxData, PluginGeneratedFile, PluginResult};
use cairo_lang_filesystem::span::TextWidth;
use cairo_lang_macro::{
    AllocationContext, Diagnostic, TokenStream, TokenStreamMetadata, TokenTree,
};
use cairo_lang_syntax::attribute::structured::{AttributeArgVariant, AttributeStructurize};
use cairo_lang_syntax::node::ast::{Expr, PathSegment};
use cairo_lang_syntax::node::db::SyntaxGroup;
use cairo_lang_syntax::node::helpers::QueryAttrs;
use cairo_lang_syntax::node::{ast, Terminal, TypedStablePtr, TypedSyntaxNode};
use convert_case::{Case, Casing};
use itertools::Itertools;

impl ProcMacroHostPlugin {
    /// Handle `#[derive(...)]` attribute.
    ///
    /// Returns a list of expansions that this plugin should apply.
    fn parse_derive(&self, db: &dyn SyntaxGroup, item_ast: ast::ModuleItem) -> Vec<ProcMacroId> {
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
                let path = path.elements(db);
                let path = path.last()?;
                let PathSegment::Simple(segment) = path else {
                    return None;
                };
                let ident = segment.ident(db);
                let value = ident.text(db).to_string();

                self.find_expansion(&Expansion::new(
                    value.to_case(Case::Snake),
                    ExpansionKind::Derive,
                ))
            })
            .collect_vec()
    }

    pub fn expand_derives(
        &self,
        db: &dyn SyntaxGroup,
        item_ast: ast::ModuleItem,
        stream_metadata: TokenStreamMetadata,
    ) -> Option<PluginResult> {
        let stable_ptr = item_ast.clone().stable_ptr().untyped();
        let mut token_stream_builder = TokenStreamBuilder::new(db);
        token_stream_builder.add_node(item_ast.as_syntax_node());
        token_stream_builder.with_metadata(stream_metadata.clone());
        let mut aux_data = EmittedAuxData::default();
        let mut all_diagnostics: Vec<Diagnostic> = Vec::new();

        // All derives to be applied.
        let derives = self.parse_derive(db, item_ast.clone());
        let any_derives = !derives.is_empty();

        let ctx = AllocationContext::default();
        let mut derived_code = PatchBuilder::new(db, &item_ast);
        let mut code_mappings = Vec::new();
        let mut current_width = TextWidth::default();

        for derive in derives.iter() {
            let token_stream = token_stream_builder.build(&ctx);
            let result = self.instance(derive.package_id).generate_code(
                derive.expansion.name.clone(),
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

            let mut mappings = generate_code_mappings(&result.token_stream);
            for mapping in &mut mappings {
                mapping.span.start = mapping.span.start.add_width(current_width);
                mapping.span.end = mapping.span.end.add_width(current_width);
            }
            code_mappings.extend(mappings);
            current_width = current_width + TextWidth::from_str(&result.token_stream.to_string());

            for token in result.token_stream.tokens {
                match token {
                    TokenTree::Ident(token) => {
                        derived_code.add_str(token.content.as_ref());
                    }
                }
            }
        }

        if any_derives {
            let derived_code = derived_code.build().0;
            return Some(PluginResult {
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
                        .map(|derive| derive.expansion.name.to_string())
                        .join("`, `");
                    let note = format!("this error originates in {msg}: `{derive_names}`");
                    Some(PluginGeneratedFile {
                        name: "proc_macro_derive".into(),
                        code_mappings,
                        content: derived_code,
                        aux_data: if aux_data.is_empty() {
                            None
                        } else {
                            Some(DynGeneratedFileAuxData::new(aux_data))
                        },
                        diagnostics_note: Some(note),
                    })
                },
                diagnostics: into_cairo_diagnostics(all_diagnostics, stable_ptr),
                // Note that we don't remove the original item here, unlike for attributes.
                // We do not add the original code to the generated file either.
                remove_original_item: false,
            });
        }

        None
    }
}
