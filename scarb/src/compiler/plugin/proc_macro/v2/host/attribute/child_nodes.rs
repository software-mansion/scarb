use cairo_lang_syntax::node::ast::{
    Attribute, FunctionWithBody, ItemEnum, ItemExternFunction, ItemExternType, ItemImpl,
    ItemModule, ItemStruct, ItemTrait,
};
use cairo_lang_syntax::node::db::SyntaxGroup;
use cairo_lang_syntax::node::{SyntaxNode, TypedSyntaxNode};

pub trait ItemWithAttributes {
    fn item_attributes(&self, db: &dyn SyntaxGroup) -> Vec<Attribute>;
}

pub trait ChildNodesWithoutAttributes {
    fn child_nodes_without_attributes(
        &self,
        db: &dyn SyntaxGroup,
    ) -> impl Iterator<Item = SyntaxNode>;
}

macro_rules! impl_child_nodes_without_attributes {
    ($t:ty, [$($child:ident),* $(,)?]) => {
        impl ChildNodesWithoutAttributes for $t {
            fn child_nodes_without_attributes(
                &self,
                db: &dyn SyntaxGroup,
            ) -> impl Iterator<Item = SyntaxNode> {
                [
                    $(self.$child(db).as_syntax_node()),*
                ].into_iter()
            }
        }
    };
}

macro_rules! impl_item_with_attributes {
    ($t:ty) => {
        impl ItemWithAttributes for $t {
            fn item_attributes(&self, db: &dyn SyntaxGroup) -> Vec<Attribute> {
                self.attributes(db).elements(db)
            }
        }
    };
}

impl_item_with_attributes!(ItemTrait);
impl_child_nodes_without_attributes!(
    ItemTrait,
    [visibility, trait_kw, name, generic_params, body]
);

impl_item_with_attributes!(ItemImpl);
impl_child_nodes_without_attributes!(
    ItemImpl,
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

impl_item_with_attributes!(ItemModule);
impl_child_nodes_without_attributes!(ItemModule, [visibility, module_kw, name, body]);

impl_item_with_attributes!(FunctionWithBody);
impl_child_nodes_without_attributes!(FunctionWithBody, [visibility, declaration, body]);

impl_item_with_attributes!(ItemExternFunction);
impl_child_nodes_without_attributes!(
    ItemExternFunction,
    [visibility, extern_kw, declaration, semicolon]
);

impl_item_with_attributes!(ItemExternType);
impl_child_nodes_without_attributes!(
    ItemExternType,
    [
        visibility,
        extern_kw,
        type_kw,
        name,
        generic_params,
        semicolon
    ]
);

impl_item_with_attributes!(ItemStruct);
impl_child_nodes_without_attributes!(
    ItemStruct,
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

impl_item_with_attributes!(ItemEnum);
impl_child_nodes_without_attributes!(
    ItemEnum,
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
