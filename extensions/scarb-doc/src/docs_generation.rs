use crate::location_links::DocLocationLink;
use crate::types::{
    Constant, Enum, ExternFunction, ExternType, FreeFunction, Impl, ImplAlias, ImplConstant,
    ImplFunction, ImplType, Member, Module, Struct, Trait, TraitConstant, TraitFunction, TraitType,
    TypeAlias, Variant,
};
use cairo_lang_doc::parser::DocumentationCommentToken;

pub mod markdown;

#[derive(Default)]
struct TopLevelItems<'a> {
    pub modules: Vec<&'a Module>,
    pub constants: Vec<&'a Constant>,
    pub free_functions: Vec<&'a FreeFunction>,
    pub structs: Vec<&'a Struct>,
    pub enums: Vec<&'a Enum>,
    pub type_aliases: Vec<&'a TypeAlias>,
    pub impl_aliases: Vec<&'a ImplAlias>,
    pub traits: Vec<&'a Trait>,
    pub impls: Vec<&'a Impl>,
    pub extern_types: Vec<&'a ExternType>,
    pub extern_functions: Vec<&'a ExternFunction>,
}

// Trait for items with no descendants.
// Used to enforce constraints on generic implementations of traits like `MarkdownDocItem`.

trait PrimitiveDocItem: DocItem {}

impl PrimitiveDocItem for Constant {}
impl PrimitiveDocItem for ExternFunction {}
impl PrimitiveDocItem for ExternType {}
impl PrimitiveDocItem for FreeFunction {}
impl PrimitiveDocItem for ImplAlias {}
impl PrimitiveDocItem for TypeAlias {}

trait SubPathDocItem: DocItem {}

impl SubPathDocItem for Member {}
impl SubPathDocItem for Variant {}
impl SubPathDocItem for TraitFunction {}
impl SubPathDocItem for ImplFunction {}
impl SubPathDocItem for ImplType {}
impl SubPathDocItem for ImplConstant {}
impl SubPathDocItem for TraitConstant {}
impl SubPathDocItem for TraitType {}

// Trait for items that have their own documentation page.
// Used to enforce constraints on generic implementations of traits like `TopLevelMarkdownDocItem`.
trait TopLevelDocItem: DocItem {}

impl TopLevelDocItem for Constant {}
impl TopLevelDocItem for Enum {}
impl TopLevelDocItem for ExternFunction {}
impl TopLevelDocItem for ExternType {}
impl TopLevelDocItem for FreeFunction {}
impl TopLevelDocItem for Impl {}
impl TopLevelDocItem for ImplAlias {}
impl TopLevelDocItem for Module {}
impl TopLevelDocItem for Struct {}
impl TopLevelDocItem for Trait {}
impl TopLevelDocItem for TypeAlias {}

// Wrapper trait over a documentable item to hide implementation details of the item type.
trait DocItem {
    const HEADER: &'static str;

    fn name(&self) -> &str;
    fn doc(&self) -> &Option<Vec<DocumentationCommentToken>>;
    fn signature(&self) -> &Option<String>;
    fn full_path(&self) -> &str;
    fn doc_location_links(&self) -> &Vec<DocLocationLink>;
    fn markdown_formatted_path(&self) -> String;
}

macro_rules! impl_doc_item {
    ($t:ty, $name:expr) => {
        impl DocItem for $t {
            const HEADER: &'static str = $name;

            fn name(&self) -> &str {
                &self.item_data.name
            }

            fn doc(&self) -> &Option<Vec<DocumentationCommentToken>> {
                &self.item_data.doc
            }

            fn signature(&self) -> &Option<String> {
                &self.item_data.signature
            }

            fn full_path(&self) -> &str {
                &self.item_data.full_path
            }

            fn doc_location_links(&self) -> &Vec<DocLocationLink> {
                &self.item_data.doc_location_links
            }

            fn markdown_formatted_path(&self) -> String {
                self.full_path().replace("::", "-")
            }
        }
    };
}

impl_doc_item!(Constant, "Constants");
impl_doc_item!(Enum, "Enums");
impl_doc_item!(ExternFunction, "Extern functions");
impl_doc_item!(ExternType, "Extern types");
impl_doc_item!(FreeFunction, "Free functions");
impl_doc_item!(Impl, "Impls");
impl_doc_item!(ImplAlias, "Impl aliases");
impl_doc_item!(ImplConstant, "Impl constants");
impl_doc_item!(ImplFunction, "Impl functions");
impl_doc_item!(ImplType, "Impl types");
impl_doc_item!(Member, "Members");
impl_doc_item!(Module, "Modules");
impl_doc_item!(Struct, "Structs");
impl_doc_item!(Trait, "Traits");
impl_doc_item!(TraitConstant, "Trait constants");
impl_doc_item!(TraitType, "Trait types");
impl_doc_item!(TraitFunction, "Trait functions");
impl_doc_item!(TypeAlias, "Type aliases");
impl_doc_item!(Variant, "Variants");
