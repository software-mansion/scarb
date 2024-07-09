// TODO(drknzz): Remove when not needed.
#![allow(dead_code)]

use itertools::Itertools;
use serde::Serialize;

use cairo_lang_defs::db::DefsGroup;
use cairo_lang_defs::diagnostic_utils::StableLocation;
use cairo_lang_defs::ids::{
    ConstantId, EnumId, ExternFunctionId, ExternTypeId, FreeFunctionId, ImplAliasId,
    ImplConstantDefId, ImplDefId, ImplFunctionId, ImplItemId, ImplTypeDefId, LookupItemId,
    MemberId, ModuleId, ModuleItemId, ModuleTypeAliasId, NamedLanguageElementId, StructId,
    TopLevelLanguageElementId, TraitConstantId, TraitFunctionId, TraitId, TraitItemId, TraitTypeId,
    VariantId,
};
use cairo_lang_doc::db::DocGroup;
use cairo_lang_filesystem::ids::CrateId;
use cairo_lang_semantic::db::SemanticGroup;
use cairo_lang_syntax::node::ast;
use cairo_lang_syntax::node::db::SyntaxGroup;
use cairo_lang_syntax::node::kind::SyntaxKind;
use cairo_lang_utils::Upcast;

use crate::db::ScarbDocDatabase;

#[derive(Serialize, Clone)]
pub struct Crate {
    pub root_module: Module,
}

impl Crate {
    pub fn new(db: &ScarbDocDatabase, crate_id: CrateId) -> Self {
        Self {
            root_module: Module::new(db, ModuleId::CrateRoot(crate_id)),
        }
    }
}

#[derive(Serialize, Clone)]
pub struct Module {
    #[serde(skip)]
    pub module_id: ModuleId,
    pub item_data: ItemData,

    pub submodules: Vec<Module>,
    pub constants: Vec<Constant>,
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
    pub fn new(db: &ScarbDocDatabase, module_id: ModuleId) -> Self {
        // FIXME: compiler doesn't support fetching root crate doc
        let item_data = match module_id {
            ModuleId::CrateRoot(crate_id) => ItemData {
                name: crate_id.name(db).to_string(),
                doc: None,
                signature: None,
                full_path: module_id.full_path(db),
            },
            ModuleId::Submodule(submodule_id) => ItemData::new_without_signature(
                db,
                submodule_id,
                LookupItemId::ModuleItem(ModuleItemId::Submodule(submodule_id)),
            ),
        };

        let module_constants = db.module_constants(module_id).unwrap();
        let constants = module_constants
            .iter()
            .map(|(id, _)| Constant::new(db, *id))
            .collect();

        let module_free_functions = db.module_free_functions(module_id).unwrap();
        let free_functions = module_free_functions
            .iter()
            .map(|(id, _)| FreeFunction::new(db, *id))
            .collect();

        let module_structs = db.module_structs(module_id).unwrap();
        let structs = module_structs
            .iter()
            .map(|(id, _)| Struct::new(db, *id))
            .collect();

        let module_enums = db.module_enums(module_id).unwrap();
        let enums = module_enums
            .iter()
            .map(|(id, _)| Enum::new(db, *id))
            .collect();

        let module_type_aliases = db.module_type_aliases(module_id).unwrap();
        let type_aliases = module_type_aliases
            .iter()
            .map(|(id, _)| TypeAlias::new(db, *id))
            .collect();

        let module_impl_aliases = db.module_impl_aliases(module_id).unwrap();
        let impl_aliases = module_impl_aliases
            .iter()
            .map(|(id, _)| ImplAlias::new(db, *id))
            .collect();

        let module_traits = db.module_traits(module_id).unwrap();
        let traits = module_traits
            .iter()
            .map(|(id, _)| Trait::new(db, *id))
            .collect();

        let module_impls = db.module_impls(module_id).unwrap();
        let impls = module_impls
            .iter()
            .map(|(id, _)| Impl::new(db, *id))
            .collect();

        let module_extern_types = db.module_extern_types(module_id).unwrap();
        let extern_types = module_extern_types
            .iter()
            .map(|(id, _)| ExternType::new(db, *id))
            .collect();

        let module_extern_functions = db.module_extern_functions(module_id).unwrap();
        let extern_functions = module_extern_functions
            .iter()
            .map(|(id, _)| ExternFunction::new(db, *id))
            .collect();

        let module_submodules = db.module_submodules(module_id).unwrap();
        let submodules = module_submodules
            .iter()
            .map(|(id, _)| Self::new(db, ModuleId::Submodule(*id)))
            .collect();

        Self {
            module_id,
            item_data,
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
        }
    }
}

#[derive(Serialize, Clone)]
pub struct ItemData {
    pub name: String,
    pub doc: Option<String>,
    pub signature: Option<String>,
    pub full_path: String,
}

impl ItemData {
    pub fn new(
        db: &ScarbDocDatabase,
        id: impl TopLevelLanguageElementId,
        lookup_item_id: LookupItemId,
    ) -> Self {
        Self {
            name: id.name(db).into(),
            doc: db.get_item_documentation(lookup_item_id),
            signature: Some(db.get_item_signature(lookup_item_id)),

            full_path: id.full_path(db),
        }
    }

    pub fn new_without_signature(
        db: &ScarbDocDatabase,
        id: impl TopLevelLanguageElementId,
        lookup_item_id: LookupItemId,
    ) -> Self {
        Self {
            name: id.name(db).into(),
            doc: db.get_item_documentation(lookup_item_id),
            signature: None,
            full_path: id.full_path(db),
        }
    }
}

#[derive(Serialize, Clone)]
pub struct Constant {
    #[serde(skip)]
    pub id: ConstantId,
    #[serde(skip)]
    pub node: ast::ItemConstantPtr,

    pub item_data: ItemData,
}

impl Constant {
    pub fn new(db: &ScarbDocDatabase, id: ConstantId) -> Self {
        let node = id.stable_ptr(db);
        Self {
            id,
            node,
            item_data: ItemData::new(db, id, LookupItemId::ModuleItem(ModuleItemId::Constant(id))),
        }
    }
}

#[derive(Serialize, Clone)]
pub struct FreeFunction {
    #[serde(skip)]
    pub id: FreeFunctionId,
    #[serde(skip)]
    pub node: ast::FunctionWithBodyPtr,

    pub item_data: ItemData,
}

impl FreeFunction {
    pub fn new(db: &ScarbDocDatabase, id: FreeFunctionId) -> Self {
        let node = id.stable_ptr(db);
        Self {
            id,
            node,
            item_data: ItemData::new(
                db,
                id,
                LookupItemId::ModuleItem(ModuleItemId::FreeFunction(id)),
            ),
        }
    }
}

#[derive(Serialize, Clone)]
pub struct Struct {
    #[serde(skip)]
    pub id: StructId,
    #[serde(skip)]
    pub node: ast::ItemStructPtr,

    pub members: Vec<Member>,

    pub item_data: ItemData,
}

impl Struct {
    pub fn new(db: &ScarbDocDatabase, id: StructId) -> Self {
        let members = db.struct_members(id).unwrap();

        let item_data = ItemData::new_without_signature(
            db,
            id,
            LookupItemId::ModuleItem(ModuleItemId::Struct(id)),
        );

        let members = members
            .iter()
            .map(|(_name, semantic_member)| {
                Member::new(db, semantic_member.id, item_data.full_path.clone())
            })
            .collect::<Vec<_>>();

        let node = id.stable_ptr(db);
        Self {
            id,
            node,
            members,
            item_data,
        }
    }
}

#[derive(Serialize, Clone)]
pub struct Member {
    #[serde(skip)]
    pub id: MemberId,
    #[serde(skip)]
    pub node: ast::MemberPtr,

    pub item_data: ItemData,
}

impl Member {
    pub fn new(db: &ScarbDocDatabase, id: MemberId, struct_full_path: String) -> Self {
        let node = id.stable_ptr(db);
        let stable_location = StableLocation::new(node.0);

        let name = id.name(db).into();
        // TODO: Replace with `id.full_path(db)` after it is fixed in the compiler.
        let full_path = format!("{}::{}", struct_full_path, name);

        let item_data = ItemData {
            name,
            doc: get_item_documentation(db.upcast(), &stable_location),
            signature: None,
            full_path,
        };

        Self {
            id,
            node,
            item_data,
        }
    }
}

#[derive(Serialize, Clone)]
pub struct Enum {
    #[serde(skip)]
    pub id: EnumId,
    #[serde(skip)]
    pub node: ast::ItemEnumPtr,

    pub variants: Vec<Variant>,

    pub item_data: ItemData,
}

impl Enum {
    pub fn new(db: &ScarbDocDatabase, id: EnumId) -> Self {
        let variants = db.enum_variants(id).unwrap();
        let item_data = ItemData::new_without_signature(
            db,
            id,
            LookupItemId::ModuleItem(ModuleItemId::Enum(id)),
        );

        let variants = variants
            .iter()
            .map(|(_name, variant_id)| Variant::new(db, *variant_id, item_data.full_path.clone()))
            .collect::<Vec<_>>();

        let node = id.stable_ptr(db);
        Self {
            id,
            node,
            variants,
            item_data,
        }
    }
}

#[derive(Serialize, Clone)]
pub struct Variant {
    #[serde(skip)]
    pub id: VariantId,
    #[serde(skip)]
    pub node: ast::VariantPtr,

    pub item_data: ItemData,
}

impl Variant {
    pub fn new(db: &ScarbDocDatabase, id: VariantId, enum_full_path: String) -> Self {
        let node = id.stable_ptr(db);
        let stable_location = StableLocation::new(node.0);

        let name = id.name(db).into();
        // TODO: Replace with `id.full_path(db)` after it is fixed in the compiler.
        let full_path = format!("{}::{}", enum_full_path, name);

        let item_data = ItemData {
            name,
            doc: get_item_documentation(db, &stable_location),
            signature: None,
            full_path,
        };

        Self {
            id,
            node,
            item_data,
        }
    }
}

#[derive(Serialize, Clone)]
pub struct TypeAlias {
    #[serde(skip)]
    pub id: ModuleTypeAliasId,
    #[serde(skip)]
    pub node: ast::ItemTypeAliasPtr,

    pub item_data: ItemData,
}

impl TypeAlias {
    pub fn new(db: &ScarbDocDatabase, id: ModuleTypeAliasId) -> Self {
        let node = id.stable_ptr(db);
        Self {
            id,
            node,
            item_data: ItemData::new(
                db,
                id,
                LookupItemId::ModuleItem(ModuleItemId::TypeAlias(id)),
            ),
        }
    }
}

#[derive(Serialize, Clone)]
pub struct ImplAlias {
    #[serde(skip)]
    pub id: ImplAliasId,
    #[serde(skip)]
    pub node: ast::ItemImplAliasPtr,

    pub item_data: ItemData,
}

impl ImplAlias {
    pub fn new(db: &ScarbDocDatabase, id: ImplAliasId) -> Self {
        let node = id.stable_ptr(db);
        Self {
            id,
            node,
            item_data: ItemData::new(
                db,
                id,
                LookupItemId::ModuleItem(ModuleItemId::ImplAlias(id)),
            ),
        }
    }
}

#[derive(Serialize, Clone)]
pub struct Trait {
    #[serde(skip)]
    pub id: TraitId,
    #[serde(skip)]
    pub node: ast::ItemTraitPtr,

    pub trait_constants: Vec<TraitConstant>,
    pub trait_types: Vec<TraitType>,
    pub trait_functions: Vec<TraitFunction>,

    pub item_data: ItemData,
}

impl Trait {
    pub fn new(db: &ScarbDocDatabase, id: TraitId) -> Self {
        let item_data = ItemData::new(db, id, LookupItemId::ModuleItem(ModuleItemId::Trait(id)));
        let full_path_to_trait = item_data
            .full_path
            .strip_suffix(item_data.name.as_str())
            .unwrap()
            .to_string();

        let trait_constants = db.trait_constants(id).unwrap();
        let trait_constants = trait_constants
            .iter()
            .map(|(_name, trait_constant_id)| {
                TraitConstant::new(db, *trait_constant_id, full_path_to_trait.clone())
            })
            .collect::<Vec<_>>();

        let trait_types = db.trait_types(id).unwrap();
        let trait_types = trait_types
            .iter()
            .map(|(_name, trait_type_id)| {
                TraitType::new(db, *trait_type_id, full_path_to_trait.clone())
            })
            .collect::<Vec<_>>();

        let trait_functions = db.trait_functions(id).unwrap();
        let trait_functions = trait_functions
            .iter()
            .map(|(_name, trait_function_id)| TraitFunction::new(db, *trait_function_id))
            .collect::<Vec<_>>();

        let node = id.stable_ptr(db);
        Self {
            id,
            node,
            trait_constants,
            trait_types,
            trait_functions,
            item_data,
        }
    }
}

#[derive(Serialize, Clone)]
pub struct TraitConstant {
    #[serde(skip)]
    pub id: TraitConstantId,
    #[serde(skip)]
    pub node: ast::TraitItemConstantPtr,

    pub item_data: ItemData,
}

impl TraitConstant {
    pub fn new(db: &ScarbDocDatabase, id: TraitConstantId, full_path_to_trait: String) -> Self {
        let node = id.stable_ptr(db);

        // FIXME: compiler returns empty string for a signature
        let mut item_data =
            ItemData::new(db, id, LookupItemId::TraitItem(TraitItemId::Constant(id)));
        // TODO: introduce proper fix in compiler
        item_data.full_path = full_path_to_trait + &item_data.full_path;

        Self {
            id,
            node,
            item_data,
        }
    }
}

#[derive(Serialize, Clone)]
pub struct TraitType {
    #[serde(skip)]
    pub id: TraitTypeId,
    #[serde(skip)]
    pub node: ast::TraitItemTypePtr,

    pub item_data: ItemData,
}

impl TraitType {
    pub fn new(db: &ScarbDocDatabase, id: TraitTypeId, full_path_to_trait: String) -> Self {
        let node = id.stable_ptr(db);

        // FIXME: compiler returns empty string for a signature
        let mut item_data = ItemData::new(db, id, LookupItemId::TraitItem(TraitItemId::Type(id)));
        // TODO: introduce proper fix in compiler
        item_data.full_path = full_path_to_trait + &item_data.full_path;

        Self {
            id,
            node,
            item_data,
        }
    }
}

#[derive(Serialize, Clone)]
pub struct TraitFunction {
    #[serde(skip)]
    pub id: TraitFunctionId,
    #[serde(skip)]
    pub node: ast::TraitItemFunctionPtr,

    pub item_data: ItemData,
}

impl TraitFunction {
    pub fn new(db: &ScarbDocDatabase, id: TraitFunctionId) -> Self {
        let node = id.stable_ptr(db);

        Self {
            id,
            node,
            item_data: ItemData::new(db, id, LookupItemId::TraitItem(TraitItemId::Function(id))),
        }
    }
}

#[derive(Serialize, Clone)]
pub struct Impl {
    #[serde(skip)]
    pub id: ImplDefId,
    #[serde(skip)]
    pub node: ast::ItemImplPtr,

    pub impl_types: Vec<ImplType>,
    pub impl_constants: Vec<ImplConstant>,
    pub impl_functions: Vec<ImplFunction>,

    pub item_data: ItemData,
}

impl Impl {
    pub fn new(db: &ScarbDocDatabase, id: ImplDefId) -> Self {
        let item_data = ItemData::new(db, id, LookupItemId::ModuleItem(ModuleItemId::Impl(id)));
        let full_path_to_impl = item_data
            .full_path
            .strip_suffix(item_data.name.as_str())
            .unwrap()
            .to_string();

        let impl_types = db.impl_types(id).unwrap();
        let impl_types = impl_types
            .iter()
            .map(|(id, _)| ImplType::new(db, *id, full_path_to_impl.clone()))
            .collect::<Vec<_>>();

        let impl_constants = db.impl_constants(id).unwrap();
        let impl_constants = impl_constants
            .iter()
            .map(|(id, _)| ImplConstant::new(db, *id, full_path_to_impl.clone()))
            .collect::<Vec<_>>();

        let impl_functions = db.impl_functions(id).unwrap();
        let impl_functions = impl_functions
            .iter()
            .map(|(_name, id)| ImplFunction::new(db, *id))
            .collect::<Vec<_>>();

        let node = id.stable_ptr(db);
        Self {
            id,
            node,
            impl_types,
            impl_constants,
            impl_functions,
            item_data,
        }
    }
}

#[derive(Serialize, Clone)]
pub struct ImplType {
    #[serde(skip)]
    pub id: ImplTypeDefId,
    #[serde(skip)]
    pub node: ast::ItemTypeAliasPtr,

    pub item_data: ItemData,
}

impl ImplType {
    pub fn new(db: &ScarbDocDatabase, id: ImplTypeDefId, full_path_to_impl: String) -> Self {
        let node = id.stable_ptr(db);

        let mut item_data = ItemData::new(db, id, LookupItemId::ImplItem(ImplItemId::Type(id)));
        // TODO: introduce proper fix in compiler
        item_data.full_path = full_path_to_impl + &item_data.full_path;

        Self {
            id,
            node,
            item_data,
        }
    }
}

#[derive(Serialize, Clone)]
pub struct ImplConstant {
    #[serde(skip)]
    pub id: ImplConstantDefId,
    #[serde(skip)]
    pub node: ast::ItemConstantPtr,

    pub item_data: ItemData,
}

impl ImplConstant {
    pub fn new(db: &ScarbDocDatabase, id: ImplConstantDefId, full_path_to_impl: String) -> Self {
        let node = id.stable_ptr(db);

        let mut item_data = ItemData::new(db, id, LookupItemId::ImplItem(ImplItemId::Constant(id)));
        // TODO: introduce proper fix in compiler
        item_data.full_path = full_path_to_impl + &item_data.full_path;

        Self {
            id,
            node,
            item_data,
        }
    }
}

#[derive(Serialize, Clone)]
pub struct ImplFunction {
    #[serde(skip)]
    pub id: ImplFunctionId,
    #[serde(skip)]
    pub node: ast::FunctionWithBodyPtr,

    pub item_data: ItemData,
}

impl ImplFunction {
    pub fn new(db: &ScarbDocDatabase, id: ImplFunctionId) -> Self {
        let node = id.stable_ptr(db);

        Self {
            id,
            node,
            item_data: ItemData::new(db, id, LookupItemId::ImplItem(ImplItemId::Function(id))),
        }
    }
}

#[derive(Serialize, Clone)]
pub struct ExternType {
    #[serde(skip)]
    pub id: ExternTypeId,
    #[serde(skip)]
    pub node: ast::ItemExternTypePtr,

    pub item_data: ItemData,
}

impl ExternType {
    pub fn new(db: &ScarbDocDatabase, id: ExternTypeId) -> Self {
        let node = id.stable_ptr(db);
        Self {
            id,
            node,
            item_data: ItemData::new(
                db,
                id,
                LookupItemId::ModuleItem(ModuleItemId::ExternType(id)),
            ),
        }
    }
}

#[derive(Serialize, Clone)]
pub struct ExternFunction {
    #[serde(skip)]
    pub id: ExternFunctionId,
    #[serde(skip)]
    pub node: ast::ItemExternFunctionPtr,

    pub item_data: ItemData,
}

impl ExternFunction {
    pub fn new(db: &ScarbDocDatabase, id: ExternFunctionId) -> Self {
        let node = id.stable_ptr(db);
        Self {
            id,
            node,
            item_data: ItemData::new(
                db,
                id,
                LookupItemId::ModuleItem(ModuleItemId::ExternFunction(id)),
            ),
        }
    }
}

// TODO: This function is temporarily copied until further modifications in cairo compiler are done.
fn get_item_documentation(db: &dyn DefsGroup, stable_location: &StableLocation) -> Option<String> {
    let doc = stable_location.syntax_node(db).get_text(db.upcast());
    let doc = doc
        .lines()
        .take_while_ref(|line| {
            !line
                .trim_start()
                .chars()
                .next()
                .map_or(false, |c| c.is_alphabetic())
        })
        .filter_map(|line| {
            let dedent = line.trim_start();
            for prefix in ["///", "//!"] {
                if let Some(content) = dedent.strip_prefix(prefix) {
                    return Some(content.strip_prefix(' ').unwrap_or(content));
                }
            }
            None
        })
        .collect::<Vec<&str>>();
    (!doc.is_empty()).then(|| doc.join("\n"))
}

// TODO: This function is temporarily copied until further modifications in cairo compiler are done.
fn get_item_signature(db: &dyn DefsGroup, stable_location: &StableLocation) -> String {
    let syntax_node = stable_location.syntax_node(db);
    let definition = match syntax_node.green_node(db.upcast()).kind {
        SyntaxKind::ItemConstant
        | SyntaxKind::TraitItemFunction
        | SyntaxKind::ItemTypeAlias
        | SyntaxKind::ItemImplAlias => syntax_node.clone().get_text_without_trivia(db.upcast()),
        SyntaxKind::FunctionWithBody | SyntaxKind::ItemExternFunction => {
            let children =
                <dyn DefsGroup as Upcast<dyn SyntaxGroup>>::upcast(db).get_children(syntax_node);
            children[1..]
                .iter()
                .map_while(|node| {
                    let kind = node.kind(db.upcast());
                    (kind != SyntaxKind::ExprBlock
                        && kind != SyntaxKind::ImplBody
                        && kind != SyntaxKind::TraitBody)
                        .then_some(
                            if kind == SyntaxKind::VisibilityPub
                                || kind == SyntaxKind::TerminalExtern
                            {
                                node.clone()
                                    .get_text_without_trivia(db.upcast())
                                    .trim()
                                    .to_owned()
                                    + " "
                            } else {
                                node.clone()
                                    .get_text_without_trivia(db.upcast())
                                    .lines()
                                    .map(|line| line.trim())
                                    .collect::<Vec<&str>>()
                                    .join("")
                            },
                        )
                })
                .collect::<Vec<String>>()
                .join("")
        }
        SyntaxKind::ItemEnum | SyntaxKind::ItemExternType | SyntaxKind::ItemStruct => {
            <dyn DefsGroup as Upcast<dyn SyntaxGroup>>::upcast(db)
                .get_children(syntax_node)
                .iter()
                .skip(1)
                .map(|node| node.clone().get_text(db.upcast()))
                .collect::<Vec<String>>()
                .join("")
        }
        SyntaxKind::ItemTrait | SyntaxKind::ItemImpl => {
            let children =
                <dyn DefsGroup as Upcast<dyn SyntaxGroup>>::upcast(db).get_children(syntax_node);
            children[1..]
                .iter()
                .enumerate()
                .map_while(|(index, node)| {
                    let kind = node.kind(db.upcast());
                    if kind != SyntaxKind::ImplBody && kind != SyntaxKind::TraitBody {
                        let text = node
                            .clone()
                            .get_text_without_trivia(db.upcast())
                            .lines()
                            .map(|line| line.trim())
                            .collect::<Vec<&str>>()
                            .join("");

                        Some(
                            if index == 0 || kind == SyntaxKind::WrappedGenericParamList {
                                text
                            } else {
                                " ".to_owned() + &text
                            },
                        )
                    } else {
                        None
                    }
                })
                .collect::<Vec<String>>()
                .join("")
        }
        _ => "".to_owned(),
    };
    definition
}
