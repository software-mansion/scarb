use crate::compiler::plugin::proc_macro::v2::host::attribute::{
    AttrExpansionArgs, AttrExpansionFound, ExpandableAttrLocation,
};
use crate::compiler::plugin::proc_macro::v2::host::conversion::CallSiteLocation;
use crate::compiler::plugin::proc_macro::v2::{ProcMacroHostPlugin, TokenStreamBuilder};
use crate::compiler::plugin::proc_macro::{ExpansionKind, ExpansionQuery};
use cairo_lang_macro::AllocationContext;
use cairo_lang_syntax::attribute::structured::AttributeStructurize;
use cairo_lang_syntax::node::db::SyntaxGroup;
use cairo_lang_syntax::node::{TypedSyntaxNode, ast};

impl ProcMacroHostPlugin {
    pub(crate) fn parse_attrs(
        &self,
        db: &dyn SyntaxGroup,
        builder: &mut TokenStreamBuilder<'_>,
        attrs: Vec<ast::Attribute>,
        ctx: &AllocationContext,
    ) -> AttrExpansionFound {
        // This function parses attributes of the item,
        // checking if those attributes correspond to a procedural macro that should be fired.
        // The proc macro attribute found is removed from attributes list,
        // while other attributes are appended to the `PathBuilder` passed as an argument.

        // Note this function does not affect the executable attributes,
        // as it only pulls `ExpansionKind::Attr` from the plugin.
        // This means that executable attributes will neither be removed from the item,
        // nor will they cause the item to be rewritten.
        let mut expansion = None;
        let mut last = true;
        for attr in attrs {
            // We ensure that this flag is changed *after* the expansion is found.
            if last {
                let structured_attr = attr.clone().structurize(db);
                let found = self.find_expansion(&ExpansionQuery::with_cairo_name(
                    structured_attr.id.clone(),
                    ExpansionKind::Attr,
                ));
                if let Some(found) = found {
                    if expansion.is_none() {
                        let mut args_builder = TokenStreamBuilder::new(db);
                        args_builder.add_node(attr.arguments(db).as_syntax_node());
                        let args = args_builder.build(ctx);
                        expansion = Some(AttrExpansionArgs {
                            id: found,
                            args,
                            call_site: CallSiteLocation::new(&attr, db),
                            attribute_location: ExpandableAttrLocation::new(&attr, db),
                        });
                        // Do not add the attribute for found expansion.
                        continue;
                    } else {
                        last = false;
                    }
                }
            }
            builder.add_node(attr.as_syntax_node());
        }
        match (expansion, last) {
            (Some(args), true) => AttrExpansionFound::Last(args),
            (Some(args), false) => AttrExpansionFound::Some(args),
            (None, _) => AttrExpansionFound::None,
        }
    }
}
