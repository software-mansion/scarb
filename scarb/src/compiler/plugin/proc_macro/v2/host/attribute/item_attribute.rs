use crate::compiler::plugin::proc_macro::v2::host::aux_data::{EmittedAuxData, ProcMacroAuxData};
use crate::compiler::plugin::proc_macro::v2::host::conversion::{
    CallSiteLocation, into_cairo_diagnostics,
};
use crate::compiler::plugin::proc_macro::v2::host::generate_code_mappings;
use crate::compiler::plugin::proc_macro::v2::{
    ProcMacroHostPlugin, ProcMacroId, TokenStreamBuilder,
};
use cairo_lang_defs::plugin::{DynGeneratedFileAuxData, PluginGeneratedFile, PluginResult};
use cairo_lang_macro::{AllocationContext, TokenStream};
use cairo_lang_syntax::node::db::SyntaxGroup;
use cairo_lang_syntax::node::{TypedSyntaxNode, ast};
use smol_str::SmolStr;

impl ProcMacroHostPlugin {
    /// Find first attribute procedural macros that should be expanded.
    ///
    /// Remove the attribute from the code.
    pub(crate) fn parse_attribute(
        &self,
        db: &dyn SyntaxGroup,
        item_ast: ast::ModuleItem,
        ctx: &AllocationContext,
    ) -> (AttrExpansionFound, TokenStream) {
        let mut token_stream_builder = TokenStreamBuilder::new(db);
        let input = match item_ast.clone() {
            ast::ModuleItem::Trait(trait_ast) => {
                let attrs = trait_ast.attributes(db).elements(db);
                let expansion = self.parse_attrs(db, &mut token_stream_builder, attrs, ctx);
                token_stream_builder.add_node(trait_ast.visibility(db).as_syntax_node());
                token_stream_builder.add_node(trait_ast.trait_kw(db).as_syntax_node());
                token_stream_builder.add_node(trait_ast.name(db).as_syntax_node());
                token_stream_builder.add_node(trait_ast.generic_params(db).as_syntax_node());
                token_stream_builder.add_node(trait_ast.body(db).as_syntax_node());
                expansion
            }
            ast::ModuleItem::Impl(impl_ast) => {
                let attrs = impl_ast.attributes(db).elements(db);
                let expansion = self.parse_attrs(db, &mut token_stream_builder, attrs, ctx);
                token_stream_builder.add_node(impl_ast.visibility(db).as_syntax_node());
                token_stream_builder.add_node(impl_ast.impl_kw(db).as_syntax_node());
                token_stream_builder.add_node(impl_ast.name(db).as_syntax_node());
                token_stream_builder.add_node(impl_ast.generic_params(db).as_syntax_node());
                token_stream_builder.add_node(impl_ast.of_kw(db).as_syntax_node());
                token_stream_builder.add_node(impl_ast.trait_path(db).as_syntax_node());
                token_stream_builder.add_node(impl_ast.body(db).as_syntax_node());
                expansion
            }
            ast::ModuleItem::Module(module_ast) => {
                let attrs = module_ast.attributes(db).elements(db);
                let expansion = self.parse_attrs(db, &mut token_stream_builder, attrs, ctx);
                token_stream_builder.add_node(module_ast.visibility(db).as_syntax_node());
                token_stream_builder.add_node(module_ast.module_kw(db).as_syntax_node());
                token_stream_builder.add_node(module_ast.name(db).as_syntax_node());
                token_stream_builder.add_node(module_ast.body(db).as_syntax_node());
                expansion
            }
            ast::ModuleItem::FreeFunction(free_func_ast) => {
                let attrs = free_func_ast.attributes(db).elements(db);
                let expansion = self.parse_attrs(db, &mut token_stream_builder, attrs, ctx);
                token_stream_builder.add_node(free_func_ast.visibility(db).as_syntax_node());
                token_stream_builder.add_node(free_func_ast.declaration(db).as_syntax_node());
                token_stream_builder.add_node(free_func_ast.body(db).as_syntax_node());
                expansion
            }
            ast::ModuleItem::ExternFunction(extern_func_ast) => {
                let attrs = extern_func_ast.attributes(db).elements(db);
                let expansion = self.parse_attrs(db, &mut token_stream_builder, attrs, ctx);
                token_stream_builder.add_node(extern_func_ast.visibility(db).as_syntax_node());
                token_stream_builder.add_node(extern_func_ast.extern_kw(db).as_syntax_node());
                token_stream_builder.add_node(extern_func_ast.declaration(db).as_syntax_node());
                token_stream_builder.add_node(extern_func_ast.semicolon(db).as_syntax_node());
                expansion
            }
            ast::ModuleItem::ExternType(extern_type_ast) => {
                let attrs = extern_type_ast.attributes(db).elements(db);
                let expansion = self.parse_attrs(db, &mut token_stream_builder, attrs, ctx);
                token_stream_builder.add_node(extern_type_ast.visibility(db).as_syntax_node());
                token_stream_builder.add_node(extern_type_ast.extern_kw(db).as_syntax_node());
                token_stream_builder.add_node(extern_type_ast.type_kw(db).as_syntax_node());
                token_stream_builder.add_node(extern_type_ast.name(db).as_syntax_node());
                token_stream_builder.add_node(extern_type_ast.generic_params(db).as_syntax_node());
                token_stream_builder.add_node(extern_type_ast.semicolon(db).as_syntax_node());
                expansion
            }
            ast::ModuleItem::Struct(struct_ast) => {
                let attrs = struct_ast.attributes(db).elements(db);
                let expansion = self.parse_attrs(db, &mut token_stream_builder, attrs, ctx);
                token_stream_builder.add_node(struct_ast.visibility(db).as_syntax_node());
                token_stream_builder.add_node(struct_ast.struct_kw(db).as_syntax_node());
                token_stream_builder.add_node(struct_ast.name(db).as_syntax_node());
                token_stream_builder.add_node(struct_ast.generic_params(db).as_syntax_node());
                token_stream_builder.add_node(struct_ast.lbrace(db).as_syntax_node());
                token_stream_builder.add_node(struct_ast.members(db).as_syntax_node());
                token_stream_builder.add_node(struct_ast.rbrace(db).as_syntax_node());
                expansion
            }
            ast::ModuleItem::Enum(enum_ast) => {
                let attrs = enum_ast.attributes(db).elements(db);
                let expansion = self.parse_attrs(db, &mut token_stream_builder, attrs, ctx);
                token_stream_builder.add_node(enum_ast.visibility(db).as_syntax_node());
                token_stream_builder.add_node(enum_ast.enum_kw(db).as_syntax_node());
                token_stream_builder.add_node(enum_ast.name(db).as_syntax_node());
                token_stream_builder.add_node(enum_ast.generic_params(db).as_syntax_node());
                token_stream_builder.add_node(enum_ast.lbrace(db).as_syntax_node());
                token_stream_builder.add_node(enum_ast.variants(db).as_syntax_node());
                token_stream_builder.add_node(enum_ast.rbrace(db).as_syntax_node());
                expansion
            }
            _ => AttrExpansionFound::None,
        };
        let token_stream = token_stream_builder.build(ctx);
        (input, token_stream)
    }

    pub fn expand_attribute(
        &self,
        db: &dyn SyntaxGroup,
        input: ProcMacroId,
        last: bool,
        args: TokenStream,
        token_stream: TokenStream,
        call_site: CallSiteLocation,
    ) -> PluginResult {
        let original = token_stream.to_string();
        let result = self
            .instance(input.package_id)
            .try_v2()
            .expect("procedural macro using v1 api used in a context expecting v2 api")
            .generate_code(
                input.expansion.expansion_name.clone(),
                call_site.span,
                args,
                token_stream,
            );

        // Handle token stream.
        if result.token_stream.is_empty() {
            // Remove original code
            return PluginResult {
                diagnostics: into_cairo_diagnostics(db, result.diagnostics, call_site.stable_ptr),
                code: None,
                remove_original_item: true,
            };
        }

        // Full path markers require code modification.
        self.register_full_path_markers(input.package_id, result.full_path_markers.clone());

        // This is a minor optimization.
        // If the expanded macro attribute is the only one that will be expanded by `ProcMacroHost`
        // in this `generate_code` call (i.e. all the other macro attributes has been expanded by
        // previous calls), and the expansion did not produce any changes, we can skip rewriting the
        // expanded node by simply returning no generated code, and leaving the original item as is.
        // However, if we have other macro attributes to expand, we must rewrite the node even if no
        // changes have been produced, so that we can parse the attributes once again and expand them.
        // In essence, `code: None, remove_original_item: false` means `ProcMacroHost` will not be
        // called again for this AST item.
        // This optimization limits the number of generated nodes a bit.
        if last && result.aux_data.is_none() && original == result.token_stream.to_string() {
            return PluginResult {
                code: None,
                remove_original_item: false,
                diagnostics: into_cairo_diagnostics(db, result.diagnostics, call_site.stable_ptr),
            };
        }

        let file_name = format!("proc_{}", input.expansion.cairo_name);
        let code_mappings = generate_code_mappings(&result.token_stream);
        let content = result.token_stream.to_string();
        PluginResult {
            code: Some(PluginGeneratedFile {
                name: file_name.into(),
                code_mappings,
                content,
                diagnostics_note: Some(format!(
                    "this error originates in the attribute macro: `{}`",
                    input.expansion.cairo_name
                )),
                aux_data: result.aux_data.map(|new_aux_data| {
                    DynGeneratedFileAuxData::new(EmittedAuxData::new(ProcMacroAuxData::new(
                        new_aux_data.into(),
                        input,
                    )))
                }),
            }),
            diagnostics: into_cairo_diagnostics(db, result.diagnostics, call_site.stable_ptr),
            remove_original_item: true,
        }
    }
}

pub enum AttrExpansionFound {
    Some(AttrExpansionArgs),
    Last(AttrExpansionArgs),
    None,
}

pub struct AttrExpansionArgs {
    pub id: ProcMacroId,
    pub args: TokenStream,
    pub call_site: CallSiteLocation,
}

impl AttrExpansionFound {
    pub fn as_name(&self) -> Option<SmolStr> {
        match self {
            AttrExpansionFound::Some(args) | AttrExpansionFound::Last(args) => {
                Some(args.id.expansion.cairo_name.clone())
            }
            AttrExpansionFound::None => None,
        }
    }
}
