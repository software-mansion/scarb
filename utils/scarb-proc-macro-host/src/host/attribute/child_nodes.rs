use cairo_lang_filesystem::span::TextSpan;
use cairo_lang_syntax::node::ast::{
    Attribute, FunctionWithBody, ItemConstant, ItemEnum, ItemExternFunction, ItemExternType,
    ItemImpl, ItemImplAlias, ItemModule, ItemStruct, ItemTrait, ItemTypeAlias, ItemUse,
};
use cairo_lang_syntax::node::{SyntaxNode, TypedSyntaxNode};
use salsa::Database;

pub trait ItemWithAttributes<'db> {
    fn item_attributes(&self, db: &'db dyn Database) -> Vec<Attribute<'db>>;
    fn span_with_trivia(&self, db: &'db dyn Database) -> TextSpan;
}

pub trait ChildNodesWithoutAttributes<'db> {
    fn child_nodes_without_attributes(
        &self,
        db: &'db dyn Database,
    ) -> impl Iterator<Item = SyntaxNode<'db>>;
}

macro_rules! impl_child_nodes_without_attributes {
    ($t:ty, [$($child:ident),* $(,)?]) => {
        impl<'db> ChildNodesWithoutAttributes<'db> for $t {
            fn child_nodes_without_attributes(
                &self,
                db: &'db dyn Database,
            ) -> impl Iterator<Item = SyntaxNode<'db>> {
                [
                    $(self.$child(db).as_syntax_node()),*
                ].into_iter()
            }
        }
    };
}

macro_rules! impl_item_with_attributes {
    ($t:ty) => {
        impl<'db> ItemWithAttributes<'db> for $t {
            fn item_attributes(&self, db: &'db dyn Database) -> Vec<Attribute<'db>> {
                self.attributes(db).elements(db).collect()
            }

            fn span_with_trivia(&self, db: &'db dyn Database) -> TextSpan {
                self.as_syntax_node().span(db)
            }
        }
    };
}

impl_item_with_attributes!(ItemTrait<'db>);
impl_child_nodes_without_attributes!(
    ItemTrait<'db>,
    [visibility, trait_kw, name, generic_params, body]
);

impl_item_with_attributes!(ItemImpl<'db>);
impl_child_nodes_without_attributes!(
    ItemImpl<'db>,
    [
        visibility,
        impl_kw,
        name,
        generic_params,
        of_kw,
        trait_path,
        body
    ]
);

impl_item_with_attributes!(ItemModule<'db>);
impl_child_nodes_without_attributes!(ItemModule<'db>, [visibility, module_kw, name, body]);

impl_item_with_attributes!(FunctionWithBody<'db>);
impl_child_nodes_without_attributes!(FunctionWithBody<'db>, [visibility, declaration, body]);

impl_item_with_attributes!(ItemExternFunction<'db>);
impl_child_nodes_without_attributes!(
    ItemExternFunction<'db>,
    [visibility, extern_kw, declaration, semicolon]
);

impl_item_with_attributes!(ItemExternType<'db>);
impl_child_nodes_without_attributes!(
    ItemExternType<'db>,
    [
        visibility,
        extern_kw,
        type_kw,
        name,
        generic_params,
        semicolon
    ]
);

impl_item_with_attributes!(ItemStruct<'db>);
impl_child_nodes_without_attributes!(
    ItemStruct<'db>,
    [
        visibility,
        struct_kw,
        name,
        generic_params,
        lbrace,
        members,
        rbrace
    ]
);

impl_item_with_attributes!(ItemEnum<'db>);
impl_child_nodes_without_attributes!(
    ItemEnum<'db>,
    [
        visibility,
        enum_kw,
        name,
        generic_params,
        lbrace,
        variants,
        rbrace
    ]
);

impl_item_with_attributes!(ItemConstant<'db>);
impl_child_nodes_without_attributes!(
    ItemConstant<'db>,
    [
        visibility,
        const_kw,
        name,
        type_clause,
        eq,
        value,
        semicolon
    ]
);

impl_item_with_attributes!(ItemUse<'db>);
impl_child_nodes_without_attributes!(ItemUse<'db>, [visibility, use_kw, use_path, semicolon]);

impl_item_with_attributes!(ItemImplAlias<'db>);
impl_child_nodes_without_attributes!(
    ItemImplAlias<'db>,
    [
        visibility,
        impl_kw,
        name,
        generic_params,
        eq,
        impl_path,
        semicolon
    ]
);

impl_item_with_attributes!(ItemTypeAlias<'db>);
impl_child_nodes_without_attributes!(
    ItemTypeAlias<'db>,
    [visibility, type_kw, name, generic_params, eq, ty, semicolon]
);
