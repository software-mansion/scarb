use crate::doc_test::code_blocks::CodeBlock;
use crate::location_links::DocLocationLink;
use crate::types::module_type::Module;
use crate::types::other_types::{
    Constant, Enum, ExternFunction, ExternType, FreeFunction, Impl, ImplAlias, ImplConstant,
    ImplFunction, ImplType, MacroDeclaration, Trait, TraitConstant, TraitFunction, TraitType,
    TypeAlias, Variant,
};
use crate::types::struct_types::{Member, Struct};
use cairo_lang_doc::parser::DocumentationCommentToken;

pub mod common;
pub mod markdown;

#[derive(Default)]
struct TopLevelItems<'a, 'db> {
    pub modules: Vec<&'a Module<'db>>,
    pub constants: Vec<&'a Constant<'db>>,
    pub free_functions: Vec<&'a FreeFunction<'db>>,
    pub structs: Vec<&'a Struct<'db>>,
    pub enums: Vec<&'a Enum<'db>>,
    pub type_aliases: Vec<&'a TypeAlias<'db>>,
    pub impl_aliases: Vec<&'a ImplAlias<'db>>,
    pub traits: Vec<&'a Trait<'db>>,
    pub impls: Vec<&'a Impl<'db>>,
    pub extern_types: Vec<&'a ExternType<'db>>,
    pub extern_functions: Vec<&'a ExternFunction<'db>>,
    pub macro_declarations: Vec<&'a MacroDeclaration<'db>>,
}

// Trait for items with no descendants.
// Used to enforce constraints on generic implementations of traits like `MarkdownDocItem`.
trait PrimitiveDocItem: DocItem {}

impl PrimitiveDocItem for Constant<'_> {}
impl PrimitiveDocItem for ExternFunction<'_> {}
impl PrimitiveDocItem for ExternType<'_> {}
impl PrimitiveDocItem for FreeFunction<'_> {}
impl PrimitiveDocItem for ImplAlias<'_> {}
impl PrimitiveDocItem for TypeAlias<'_> {}
impl PrimitiveDocItem for MacroDeclaration<'_> {}

// Trait for items which file path resolutions is relative to their parent.
trait SubPathDocItem: DocItem {}

impl SubPathDocItem for Member<'_> {}
impl SubPathDocItem for Variant<'_> {}
impl SubPathDocItem for TraitFunction<'_> {}
impl SubPathDocItem for ImplFunction<'_> {}
impl SubPathDocItem for ImplType<'_> {}
impl SubPathDocItem for ImplConstant<'_> {}
impl SubPathDocItem for TraitConstant<'_> {}
impl SubPathDocItem for TraitType<'_> {}

// Trait for items that have their own documentation page.
// Used to enforce constraints on generic implementations of traits like `TopLevelMarkdownDocItem`.
pub trait TopLevelDocItem: DocItem {}

impl TopLevelDocItem for Constant<'_> {}
impl TopLevelDocItem for Enum<'_> {}
impl TopLevelDocItem for ExternFunction<'_> {}
impl TopLevelDocItem for ExternType<'_> {}
impl TopLevelDocItem for FreeFunction<'_> {}
impl TopLevelDocItem for Impl<'_> {}
impl TopLevelDocItem for ImplAlias<'_> {}
impl TopLevelDocItem for Module<'_> {}
impl TopLevelDocItem for Struct<'_> {}
impl TopLevelDocItem for Trait<'_> {}
impl TopLevelDocItem for TypeAlias<'_> {}
impl TopLevelDocItem for MacroDeclaration<'_> {}

// Wrapper trait over a documentable item to hide implementation details of the item type.
pub trait DocItem {
    const HEADER: &'static str;

    fn name(&self) -> &str;
    fn doc(&self) -> &Option<Vec<DocumentationCommentToken<'_>>>;
    fn signature(&self) -> &Option<String>;
    fn full_path(&self) -> &str;
    fn doc_location_links(&self) -> &Vec<DocLocationLink>;
    fn markdown_formatted_path(&self) -> String;
    fn group_name(&self) -> &Option<String>;
    fn code_blocks(&self) -> &Vec<CodeBlock>;
}

macro_rules! impl_doc_item {
    ($t:ty, $name:expr) => {
        impl DocItem for $t {
            const HEADER: &'static str = $name;

            fn name(&self) -> &str {
                &self.item_data.name
            }

            fn doc(&self) -> &Option<Vec<DocumentationCommentToken<'_>>> {
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

            fn group_name(&self) -> &Option<String> {
                &self.item_data.group
            }

            fn code_blocks(&self) -> &Vec<CodeBlock> {
                &self.item_data.code_blocks
            }
        }
    };
}

impl_doc_item!(Constant<'_>, "Constants");
impl_doc_item!(Enum<'_>, "Enums");
impl_doc_item!(ExternFunction<'_>, "Extern functions");
impl_doc_item!(ExternType<'_>, "Extern types");
impl_doc_item!(FreeFunction<'_>, "Free functions");
impl_doc_item!(Impl<'_>, "Impls");
impl_doc_item!(ImplAlias<'_>, "Impl aliases");
impl_doc_item!(ImplConstant<'_>, "Impl constants");
impl_doc_item!(ImplFunction<'_>, "Impl functions");
impl_doc_item!(ImplType<'_>, "Impl types");
impl_doc_item!(Member<'_>, "Members");
impl_doc_item!(Module<'_>, "Modules");
impl_doc_item!(Struct<'_>, "Structs");
impl_doc_item!(Trait<'_>, "Traits");
impl_doc_item!(TraitConstant<'_>, "Trait constants");
impl_doc_item!(TraitType<'_>, "Trait types");
impl_doc_item!(TraitFunction<'_>, "Trait functions");
impl_doc_item!(TypeAlias<'_>, "Type aliases");
impl_doc_item!(Variant<'_>, "Variants");
impl_doc_item!(MacroDeclaration<'_>, "Macro declarations");
