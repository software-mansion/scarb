use crate::types::{
    Constant, Crate, Enum, ExternFunction, ExternType, FreeFunction, Impl, ImplAlias, ImplConstant,
    ImplFunction, ImplType, Member, Module, Struct, Trait, TraitConstant, TraitFunction, TraitType,
    TypeAlias, Variant,
};
use cairo_lang_doc::db::Documentation;

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

fn collect_all_top_level_items(crate_: &Crate) -> TopLevelItems {
    let mut top_level_items = TopLevelItems::default();

    top_level_items.modules.push(&crate_.root_module);

    collect_all_top_level_items_internal(&mut top_level_items, &crate_.root_module);
    top_level_items
}

fn collect_all_top_level_items_internal<'a, 'b>(
    top_level_items: &'a mut TopLevelItems<'b>,
    module: &'b Module,
) where
    'b: 'a,
{
    let Module {
        module_id: _module_id,
        item_data: _item_data,
        submodules,
        constants,
        free_functions,
        structs,
        enums,
        type_aliases,
        impl_aliases,
        traits,
        impls,
        extern_types,
        extern_functions,
    } = &module;

    top_level_items.modules.extend(submodules);
    top_level_items.constants.extend(constants);
    top_level_items.free_functions.extend(free_functions);
    top_level_items.structs.extend(structs);
    top_level_items.enums.extend(enums);
    top_level_items.type_aliases.extend(type_aliases);
    top_level_items.impl_aliases.extend(impl_aliases);
    top_level_items.traits.extend(traits);
    top_level_items.impls.extend(impls);
    top_level_items.extern_types.extend(extern_types);
    top_level_items.extern_functions.extend(extern_functions);

    for module in submodules {
        collect_all_top_level_items_internal(top_level_items, module);
    }
}

// Trait for items with no descendants.
// Used to enforce constraints on generic implementations of traits like `MarkdownDocItem`.
trait PrimitiveDocItem: DocItem {}

impl PrimitiveDocItem for Constant {}
impl PrimitiveDocItem for ExternFunction {}
impl PrimitiveDocItem for ExternType {}
impl PrimitiveDocItem for FreeFunction {}
impl PrimitiveDocItem for ImplAlias {}
impl PrimitiveDocItem for ImplConstant {}
impl PrimitiveDocItem for ImplFunction {}
impl PrimitiveDocItem for ImplType {}
impl PrimitiveDocItem for Member {}
impl PrimitiveDocItem for TraitConstant {}
impl PrimitiveDocItem for TraitFunction {}
impl PrimitiveDocItem for TraitType {}
impl PrimitiveDocItem for TypeAlias {}
impl PrimitiveDocItem for Variant {}

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
    fn doc(&self) -> &Documentation;
    fn signature(&self) -> &Option<String>;
    fn full_path(&self) -> &str;
}

macro_rules! impl_doc_item {
    ($t:ty, $name:expr) => {
        impl DocItem for $t {
            const HEADER: &'static str = $name;

            fn name(&self) -> &str {
                &self.item_data.name
            }

            fn doc(&self) -> &Documentation {
                &self.item_data.doc
            }

            fn signature(&self) -> &Option<String> {
                &self.item_data.signature
            }

            fn full_path(&self) -> &str {
                &self.item_data.full_path
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
