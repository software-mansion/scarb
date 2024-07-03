// TODO(drknzz): Remove if not needed.
#![allow(dead_code)]

use cairo_lang_defs::db::DefsGroup;
use cairo_lang_defs::ids::{
    ConstantId, EnumId, ExternFunctionId, ExternTypeId, FreeFunctionId, ImplAliasId, ImplDefId,
    LookupItemId, ModuleId, ModuleItemId, ModuleTypeAliasId, NamedLanguageElementId, StructId,
    TopLevelLanguageElementId, TraitId, UseId,
};
use cairo_lang_doc::db::DocGroup;
use cairo_lang_filesystem::ids::CrateId;
use cairo_lang_syntax::node::{ast, TypedSyntaxNode};

#[derive(Clone, Debug)]
pub struct Crate {
    pub root_module: Module,
}

impl Crate {
    pub fn new(db: &dyn DocGroup, crate_id: CrateId) -> Self {
        Self {
            root_module: Module::new(db, ModuleId::CrateRoot(crate_id)),
        }
    }
}

#[derive(Clone, Debug)]
pub struct Module {
    pub module_id: ModuleId,
    pub full_path: String,

    pub submodules: Vec<Module>,
    pub constants: Vec<Constant>,
    pub uses: Vec<Use>,
    pub free_functions: Vec<FreeFunction>,
    pub structs: Vec<Struct>,
    pub enums: Vec<Enum>,
    pub type_aliases: Vec<TypeAlias>,
    pub impl_aliases: Vec<ImplAlias>,
    pub traits: Vec<Trait>,
    pub impls: Vec<Impl>,
    pub extern_types: Vec<ExternType>,
    pub extern_functions: Vec<ExternFunction>,
}

impl Module {
    pub fn new(db: &dyn DocGroup, module_id: ModuleId) -> Self {
        let defs_db: &dyn DefsGroup = db.upcast();
        let module_constants = defs_db.module_constants(module_id).unwrap();
        let constants = module_constants
            .iter()
            .map(|(id, node)| Constant::new(db, *id, node))
            .collect();

        let module_uses = defs_db.module_uses(module_id).unwrap();
        let uses = module_uses
            .iter()
            .map(|(id, node)| Use::new(db, *id, node))
            .collect();

        let module_free_functions = defs_db.module_free_functions(module_id).unwrap();
        let free_functions = module_free_functions
            .iter()
            .map(|(id, node)| FreeFunction::new(db, *id, node))
            .collect();

        let module_structs = defs_db.module_structs(module_id).unwrap();
        let structs = module_structs
            .iter()
            .map(|(id, node)| Struct::new(db, *id, node))
            .collect();

        let module_enums = defs_db.module_enums(module_id).unwrap();
        let enums = module_enums
            .iter()
            .map(|(id, node)| Enum::new(db, *id, node))
            .collect();

        let module_type_aliases = defs_db.module_type_aliases(module_id).unwrap();
        let type_aliases = module_type_aliases
            .iter()
            .map(|(id, node)| TypeAlias::new(db, *id, node))
            .collect();

        let module_impl_aliases = defs_db.module_impl_aliases(module_id).unwrap();
        let impl_aliases = module_impl_aliases
            .iter()
            .map(|(id, node)| ImplAlias::new(db, *id, node))
            .collect();

        let module_traits = defs_db.module_traits(module_id).unwrap();
        let traits = module_traits
            .iter()
            .map(|(id, node)| Trait::new(db, *id, node))
            .collect();

        let module_impls = defs_db.module_impls(module_id).unwrap();
        let impls = module_impls
            .iter()
            .map(|(id, node)| Impl::new(db, *id, node))
            .collect();

        let module_extern_types = defs_db.module_extern_types(module_id).unwrap();
        let extern_types = module_extern_types
            .iter()
            .map(|(id, node)| ExternType::new(db, *id, node))
            .collect();

        let module_extern_functions = defs_db.module_extern_functions(module_id).unwrap();
        let extern_functions = module_extern_functions
            .iter()
            .map(|(id, node)| ExternFunction::new(db, *id, node))
            .collect();

        let module_submodules = defs_db.module_submodules(module_id).unwrap();
        let submodules = module_submodules
            .iter()
            .map(|(id, _node)| Self::new(db, ModuleId::Submodule(*id)))
            .collect();

        Self {
            module_id,
            full_path: module_id.full_path(defs_db),
            submodules,
            constants,
            uses,
            free_functions,
            structs,
            enums,
            type_aliases,
            impl_aliases,
            traits,
            impls,
            extern_types,
            extern_functions,
        }
    }
}

#[derive(Clone, Debug)]
pub struct ItemData {
    pub name: String,
    pub full_path: String,

    pub doc: Option<String>,
    pub definition: String,

    pub text: String,
}

impl ItemData {
    pub fn new(db: &dyn DocGroup, id: ModuleItemId, node: &impl TypedSyntaxNode) -> Self {
        let defs_db = db.upcast();
        Self {
            name: id.name(defs_db).into(),
            full_path: id.full_path(defs_db),
            doc: db.get_item_documentation(LookupItemId::ModuleItem(id)),
            definition: db.get_item_signature(LookupItemId::ModuleItem(id)),
            text: node.as_syntax_node().get_text_without_trivia(db.upcast()),
        }
    }
}

#[derive(Clone, Debug)]
pub struct Constant {
    pub id: ConstantId,
    pub node: ast::ItemConstantPtr,

    pub item_data: ItemData,
}

impl Constant {
    pub fn new(db: &dyn DocGroup, id: ConstantId, node: &ast::ItemConstant) -> Self {
        Self {
            id,
            node: node.stable_ptr(),
            item_data: ItemData::new(db, ModuleItemId::Constant(id), node),
        }
    }
}

#[derive(Clone, Debug)]
pub struct Use {
    pub id: UseId,
    pub node: ast::UsePathLeafPtr,

    pub item_data: ItemData,
}

impl Use {
    pub fn new(db: &dyn DocGroup, id: UseId, node: &ast::UsePathLeaf) -> Self {
        Self {
            id,
            node: node.stable_ptr(),
            item_data: ItemData::new(db, ModuleItemId::Use(id), node),
        }
    }
}

#[derive(Clone, Debug)]
pub struct FreeFunction {
    pub id: FreeFunctionId,
    pub node: ast::FunctionWithBodyPtr,

    pub item_data: ItemData,
}

impl FreeFunction {
    pub fn new(db: &dyn DocGroup, id: FreeFunctionId, node: &ast::FunctionWithBody) -> Self {
        Self {
            id,
            node: node.stable_ptr(),
            item_data: ItemData::new(db, ModuleItemId::FreeFunction(id), node),
        }
    }
}

#[derive(Clone, Debug)]
pub struct Struct {
    pub id: StructId,
    pub node: ast::ItemStructPtr,

    pub item_data: ItemData,
}

impl Struct {
    pub fn new(db: &dyn DocGroup, id: StructId, node: &ast::ItemStruct) -> Self {
        Self {
            id,
            node: node.stable_ptr(),
            item_data: ItemData::new(db, ModuleItemId::Struct(id), node),
        }
    }
}

#[derive(Clone, Debug)]
pub struct Enum {
    pub id: EnumId,
    pub node: ast::ItemEnumPtr,

    pub item_data: ItemData,
}

impl Enum {
    pub fn new(db: &dyn DocGroup, id: EnumId, node: &ast::ItemEnum) -> Self {
        Self {
            id,
            node: node.stable_ptr(),
            item_data: ItemData::new(db, ModuleItemId::Enum(id), node),
        }
    }
}

#[derive(Clone, Debug)]
pub struct TypeAlias {
    pub id: ModuleTypeAliasId,
    pub node: ast::ItemTypeAliasPtr,

    pub item_data: ItemData,
}

impl TypeAlias {
    pub fn new(db: &dyn DocGroup, id: ModuleTypeAliasId, node: &ast::ItemTypeAlias) -> Self {
        Self {
            id,
            node: node.stable_ptr(),
            item_data: ItemData::new(db, ModuleItemId::TypeAlias(id), node),
        }
    }
}

#[derive(Clone, Debug)]
pub struct ImplAlias {
    pub id: ImplAliasId,
    pub node: ast::ItemImplAliasPtr,

    pub item_data: ItemData,
}

impl ImplAlias {
    pub fn new(db: &dyn DocGroup, id: ImplAliasId, node: &ast::ItemImplAlias) -> Self {
        Self {
            id,
            node: node.stable_ptr(),
            item_data: ItemData::new(db, ModuleItemId::ImplAlias(id), node),
        }
    }
}

#[derive(Clone, Debug)]
pub struct Trait {
    pub id: TraitId,
    pub node: ast::ItemTraitPtr,

    pub item_data: ItemData,
}

impl Trait {
    pub fn new(db: &dyn DocGroup, id: TraitId, node: &ast::ItemTrait) -> Self {
        Self {
            id,
            node: node.stable_ptr(),
            item_data: ItemData::new(db, ModuleItemId::Trait(id), node),
        }
    }
}

#[derive(Clone, Debug)]
pub struct Impl {
    pub id: ImplDefId,
    pub node: ast::ItemImplPtr,

    pub item_data: ItemData,
}

impl Impl {
    pub fn new(db: &dyn DocGroup, id: ImplDefId, node: &ast::ItemImpl) -> Self {
        Self {
            id,
            node: node.stable_ptr(),
            item_data: ItemData::new(db, ModuleItemId::Impl(id), node),
        }
    }
}

#[derive(Clone, Debug)]
pub struct ExternType {
    pub id: ExternTypeId,
    pub node: ast::ItemExternTypePtr,

    pub item_data: ItemData,
}

impl ExternType {
    pub fn new(db: &dyn DocGroup, id: ExternTypeId, node: &ast::ItemExternType) -> Self {
        Self {
            id,
            node: node.stable_ptr(),
            item_data: ItemData::new(db, ModuleItemId::ExternType(id), node),
        }
    }
}

#[derive(Clone, Debug)]
pub struct ExternFunction {
    pub id: ExternFunctionId,
    pub node: ast::ItemExternFunctionPtr,

    pub item_data: ItemData,
}

impl ExternFunction {
    pub fn new(db: &dyn DocGroup, id: ExternFunctionId, node: &ast::ItemExternFunction) -> Self {
        Self {
            id,
            node: node.stable_ptr(),
            item_data: ItemData::new(db, ModuleItemId::ExternFunction(id), node),
        }
    }
}
