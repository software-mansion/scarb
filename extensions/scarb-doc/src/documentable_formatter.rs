use crate::bounding::{
    end_bounding, extract_and_format, get_postfix, get_syntactic_mutability,
    get_syntactic_visibility, start_bounding, BoundingPostfix, BoundingType, SyntacticKind,
};
use crate::db::ScarbDocDatabase;
use cairo_lang_defs::ids::{
    ConstantId, EnumId, ExternFunctionId, ExternTypeId, FreeFunctionId, ImplAliasId,
    ImplConstantDefId, ImplDefId, ImplFunctionId, ImplTypeDefId, ModuleTypeAliasId, StructId,
    TopLevelLanguageElementId, TraitConstantId, TraitFunctionId, TraitId, TraitTypeId,
};
use cairo_lang_defs::ids::{LanguageElementId, LookupItemId, NamedLanguageElementId};
use cairo_lang_doc::documentable_item::DocumentableItemId;
use cairo_lang_semantic::db::SemanticGroup;
use cairo_lang_semantic::items::constant::ConstValue;
use cairo_lang_semantic::items::module::ModuleItemInfo;
use cairo_lang_semantic::items::visibility::Visibility;
use cairo_lang_semantic::{Expr, Signature};
use cairo_lang_syntax::node::db::SyntaxGroup;
use cairo_lang_syntax::node::kind::SyntaxKind;
use cairo_lang_syntax::node::{green, SyntaxNode};
use cairo_lang_utils::{LookupIntern, Upcast};
use smol_str::SmolStr;
use std::collections::HashMap;
use std::fmt;
use std::fmt::Write;

const INDENT: &str = "    ";

pub trait HirDisplay {
    fn hir_fmt(
        &self,
        f: &mut HirFormatter<'_>,
        item_id: DocumentableItemId,
    ) -> Result<(), fmt::Error>;

    fn get_module_item_info(item_id: DocumentableItemId, db: &ScarbDocDatabase) -> ModuleItemInfo {
        if let DocumentableItemId::LookupItem(LookupItemId::ModuleItem(module_item_id)) = item_id {
            let parent_module = module_item_id.parent_module(db);
            let item_name = module_item_id.name(db);
            db.module_item_info_by_name(parent_module, item_name.clone())
                .unwrap()
                .unwrap()
        } else {
            panic!("Expected a ModuleItem, found a different type");
        }
    }

    fn get_signature(&self, f: &mut HirFormatter<'_>, item_id: DocumentableItemId) -> String {
        self.hir_fmt(f, item_id).unwrap();
        f.buf.clone()
    }
}

pub struct HirFormatter<'a> {
    /// The database handle
    pub db: &'a ScarbDocDatabase,
    /// A buffer to intercept writes with, this allows us to track the overall size of the formatted output.
    buf: String,
    members_buff: Option<HashMap<SmolStr, String>>,
}

impl fmt::Write for HirFormatter<'_> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.buf.push_str(s);
        Ok(())
    }
}

impl<'a> HirFormatter<'a> {
    pub fn new(db: &'a ScarbDocDatabase) -> Self {
        Self {
            db,
            buf: String::new(),
            members_buff: None,
        }
    }

    pub fn new_complex(db: &'a ScarbDocDatabase) -> Self {
        Self {
            db,
            buf: String::new(),
            members_buff: Some(HashMap::new()),
        }
    }

    pub fn get_members_buff(&self) -> Option<HashMap<SmolStr, String>> {
        self.members_buff.clone()
    }
}

impl HirDisplay for EnumId {
    fn hir_fmt(
        &self,
        f: &mut HirFormatter<'_>,
        item_id: DocumentableItemId,
    ) -> Result<(), fmt::Error> {
        let module_item_info = Self::get_module_item_info(item_id, f.db);
        f.write_visibility(module_item_info.visibility)?;
        let item_name = item_id.name(f.db);
        write!(
            f,
            "{} {}: {}",
            SyntacticKind::Enum.get_syntax(),
            item_name,
            start_bounding(BoundingType::Braces)
        )?;

        let variants = f.db.enum_variants(*self).unwrap();
        variants.iter().for_each(|(name, variant_id)| {
            let variant_semantic = f.db.variant_semantic(*self, *variant_id).unwrap();
            let variant_type = variant_semantic.ty.format(f.db);
            if variant_type != "()" {
                let variant_type = extract_and_format(&variant_type);
                let variant_signature = format!("{name}: {variant_type}");
                write!(f, "\n{INDENT}{variant_signature},").unwrap();
                f.members_buff
                    .as_mut()
                    .unwrap()
                    .insert(name.clone(), format!("{name}: {variant_type}"));
            } else {
                write!(f, "\n{INDENT}{name},").unwrap();
                f.members_buff
                    .as_mut()
                    .unwrap()
                    .insert(name.clone(), format!("{}", name));
            }
        });
        write!(f, "\n{}", end_bounding(BoundingType::Braces))?;
        Ok(())
    }
}

impl HirDisplay for StructId {
    fn hir_fmt(
        &self,
        f: &mut HirFormatter<'_>,
        item_id: DocumentableItemId,
    ) -> Result<(), fmt::Error> {
        let module_item_info = Self::get_module_item_info(item_id, f.db);
        f.write_visibility(module_item_info.visibility)?;

        let semantic_members = f.db.struct_members(*self).unwrap();
        write!(
            f,
            "{} {}: {}",
            SyntacticKind::Struct.get_syntax(),
            item_id.name(f.db),
            start_bounding(BoundingType::Braces)
        )?;
        semantic_members.iter().for_each(|(_, m)| {
            let member = format!(
                "{}{}: {}",
                get_syntactic_visibility(&m.visibility),
                m.id.name(f.db),
                extract_and_format(&m.ty.format(f.db)),
            );
            write!(f, "\n{INDENT}{},", member).unwrap();
            f.members_buff
                .as_mut()
                .unwrap()
                .insert(m.id.name(f.db).clone(), member);
        });
        write!(f, "\n{}", end_bounding(BoundingType::Braces),)?;
        Ok(())
    }
}

trait FunctionSignatureWriter {
    fn write_function_signature(
        &self,
        f: &mut HirFormatter<'_>,
        signature: Signature,
        item_id: DocumentableItemId,
        syntactic_kind: SyntacticKind,
    ) -> Result<(), fmt::Error> {
        write!(
            f,
            "{} {}{}",
            syntactic_kind.get_syntax(),
            item_id.name(f.db),
            start_bounding(BoundingType::Parenthesis)
        )?;

        let mut count = signature.params.len();
        let mut postfix = ", ";
        signature.params.iter().for_each(|param| {
            if count == 1 {
                postfix = "";
            }
            let syntax_node = param
                .id
                .stable_location(f.db.upcast())
                .syntax_node(f.db.upcast());
            let type_definition = Self::get_type_clause(syntax_node, f).unwrap();

            write!(
                f,
                "{}{}{}{}",
                get_syntactic_mutability(&param.mutability),
                param.name,
                type_definition,
                postfix
            )
            .unwrap();
            count -= 1;
        });

        let return_postfix = {
            if signature.return_type.format(f.db).eq("()") {
                ""
            } else {
                &format!(
                    " {} {}",
                    get_postfix(BoundingPostfix::Arrow),
                    extract_and_format(&signature.return_type.format(f.db))
                )
            }
        };
        write!(
            f,
            "{}{}",
            end_bounding(BoundingType::Parenthesis),
            return_postfix
        )?;
        Ok(())
    }

    fn get_type_clause(syntax_node: SyntaxNode, f: &mut HirFormatter<'_>) -> Option<String> {
        let definition = String::from("<missing>");
        let children = f.db.get_children(syntax_node);
        for child in children.iter() {
            if child.kind(f.db.upcast()) == SyntaxKind::TypeClause {
                return Some(
                    child
                        .clone()
                        .get_text_without_all_comment_trivia(f.db.upcast()),
                );
            }
        }
        Some(definition)
    }
}

impl FunctionSignatureWriter for FreeFunctionId {}

impl HirDisplay for FreeFunctionId {
    fn hir_fmt(
        &self,
        f: &mut HirFormatter<'_>,
        item_id: DocumentableItemId,
    ) -> Result<(), fmt::Error> {
        let module_item_info = Self::get_module_item_info(item_id, f.db);

        f.write_visibility(module_item_info.visibility)?;
        let signature = f.db.free_function_signature(*self).unwrap();
        Self::write_function_signature(self, f, signature, item_id, SyntacticKind::Function)
    }
}

trait SyntacticWriter {
    fn write_syntactic_evaluation(
        f: &mut HirFormatter<'_>,
        item_id: DocumentableItemId,
    ) -> Result<(), fmt::Error> {
        let syntax_node = item_id
            .stable_location(f.db.upcast())
            .unwrap()
            .syntax_node(f.db.upcast());

        if matches!(
            &syntax_node.green_node(f.db.upcast()).details,
            green::GreenNodeDetails::Node { .. }
        ) {
            let mut is_after_evaluation_value = false;
            for child in f.db.get_children(syntax_node.clone()).iter() {
                let kind = child.kind(f.db);
                if !matches!(kind, SyntaxKind::Trivia) {
                    if matches!(kind, SyntaxKind::TerminalSemicolon) {
                        write!(f.buf, ";")?;
                        return Ok(());
                    }
                    if is_after_evaluation_value {
                        f.buf
                            .write_str(&SyntaxNode::get_text_without_all_comment_trivia(
                                child, f.db,
                            ))?;
                    };
                    if matches!(kind, SyntaxKind::TerminalEq) {
                        is_after_evaluation_value = true;
                    }
                }
            }
        };

        Ok(())
    }
}

impl SyntacticWriter for ConstantId {}
impl HirDisplay for ConstantId {
    fn hir_fmt(
        &self,
        f: &mut HirFormatter<'_>,
        item_id: DocumentableItemId,
    ) -> Result<(), fmt::Error> {
        let module_item_info = Self::get_module_item_info(item_id, f.db);
        f.write_visibility(module_item_info.visibility)?;
        write!(
            f,
            "{} {}: {} {} ",
            SyntacticKind::Constant.get_syntax(),
            item_id.name(f.db),
            extract_and_format(&f.db.constant_const_type(*self).unwrap().format(f.db)),
            get_postfix(BoundingPostfix::EqualSign),
        )?;
        let constant = f.db.constant_semantic_data(*self).unwrap();
        let constant_value =
            f.db.lookup_intern_const_value(f.db.constant_const_value(*self).unwrap());
        let expression = &constant.arenas.exprs[constant.value];

        match expression {
            Expr::Literal(v) => {
                write!(f, "{};", v.value)?;
            }
            Expr::FunctionCall(_) => {
                if let ConstValue::Int(value, _) = constant_value {
                    Self::write_syntactic_evaluation(f, item_id)?;
                    write!(f.buf, " // = {}", value)?;
                }
            }
            _ => {
                Self::write_syntactic_evaluation(f, item_id)?;
            }
        };
        Ok(())
    }
}

impl SyntacticWriter for ImplConstantDefId {}

impl HirDisplay for ImplConstantDefId {
    fn hir_fmt(
        &self,
        f: &mut HirFormatter<'_>,
        item_id: DocumentableItemId,
    ) -> Result<(), fmt::Error> {
        let def_value_id = f.db.impl_constant_def_value(*self).unwrap();
        write!(
            f,
            "{}: {} = ",
            item_id.name(f.db),
            extract_and_format(&def_value_id.ty(f.db).unwrap().format(f.db))
        )?;
        Self::write_syntactic_evaluation(f, item_id)?;
        Ok(())
    }
}

impl FunctionSignatureWriter for TraitFunctionId {}

impl HirDisplay for TraitFunctionId {
    fn hir_fmt(
        &self,
        f: &mut HirFormatter<'_>,
        item_id: DocumentableItemId,
    ) -> Result<(), fmt::Error> {
        let signature = f.db.trait_function_signature(*self).unwrap();
        Self::write_function_signature(self, f, signature, item_id, SyntacticKind::Function)
    }
}

impl FunctionSignatureWriter for ImplFunctionId {}

impl HirDisplay for ImplFunctionId {
    fn hir_fmt(
        &self,
        f: &mut HirFormatter<'_>,
        item_id: DocumentableItemId,
    ) -> Result<(), fmt::Error> {
        let signature = f.db.impl_function_signature(*self).unwrap();
        Self::write_function_signature(self, f, signature, item_id, SyntacticKind::Function)
    }
}

impl HirDisplay for TraitId {
    fn hir_fmt(
        &self,
        f: &mut HirFormatter<'_>,
        item_id: DocumentableItemId,
    ) -> Result<(), fmt::Error> {
        let module_item_info = Self::get_module_item_info(item_id, f.db);
        f.write_visibility(module_item_info.visibility)?;

        write!(
            f,
            "{} {}",
            SyntacticKind::Trait.get_syntax(),
            item_id.name(f.db)
        )?;
        let resolver_data = f.db.trait_resolver_data(*self).unwrap();
        let generic_params = &resolver_data.generic_params;
        if !generic_params.is_empty() {
            let mut count = generic_params.len();
            f.write_str("<")?;
            generic_params.iter().for_each(|param| {
                if count == 1 {
                    write!(f, "{}>", param.format(f.db)).unwrap();
                } else {
                    write!(f, "{}, ", param.format(f.db)).unwrap();
                    count -= 1;
                }
            });
        };
        Ok(())
    }
}

impl HirDisplay for TraitConstantId {
    fn hir_fmt(
        &self,
        f: &mut HirFormatter<'_>,
        item_id: DocumentableItemId,
    ) -> Result<(), fmt::Error> {
        write!(
            f,
            "{} {}: {};",
            SyntacticKind::Trait.get_syntax(),
            item_id.name(f.db),
            extract_and_format(&f.db.trait_constant_type(*self).unwrap().format(f.db)),
        )?;
        Ok(())
    }
}

impl HirDisplay for ImplDefId {
    fn hir_fmt(
        &self,
        f: &mut HirFormatter<'_>,
        item_id: DocumentableItemId,
    ) -> Result<(), fmt::Error> {
        let module_item_info = Self::get_module_item_info(item_id, f.db);
        f.write_visibility(module_item_info.visibility)?;
        let impl_def_trait_id = f.db.impl_def_trait(*self);
        match impl_def_trait_id {
            Ok(trait_id) => {
                let concrete_trait_id = f.db.impl_def_concrete_trait(*self).unwrap();
                let intern = concrete_trait_id.lookup_intern(f.db);

                let path = {
                    if format!(
                        "{}::{}",
                        intern
                            .trait_id
                            .parent_module(f.db)
                            .owning_crate(f.db)
                            .name(f.db),
                        trait_id.name(f.db)
                    ) == trait_id.full_path(f.db)
                    {
                        String::from(trait_id.name(f.db))
                    } else {
                        trait_id.full_path(f.db)
                    }
                };
                write!(
                    f,
                    "{} {} of {}",
                    SyntacticKind::Impl.get_syntax(),
                    item_id.name(f.db),
                    path
                )?;
                let mut count = intern.generic_args.len();
                intern.generic_args.iter().for_each(|arg| {
                    let gt = extract_and_format(&arg.format(f.db));
                    write!(
                        f,
                        "{}{}{}",
                        if count == intern.generic_args.len() {
                            "<"
                        } else {
                            ""
                        },
                        gt,
                        if count == 1 { ">;" } else { ";" }
                    )
                    .unwrap();
                    count -= 1;
                });
            }
            Err(err) => {
                println!("{:?}", err);
            }
        };
        Ok(())
    }
}

impl SyntacticWriter for ImplAliasId {}

impl HirDisplay for ImplAliasId {
    fn hir_fmt(
        &self,
        f: &mut HirFormatter<'_>,
        item_id: DocumentableItemId,
    ) -> Result<(), fmt::Error> {
        let module_item_info = Self::get_module_item_info(item_id, f.db);
        f.write_visibility(module_item_info.visibility)?;
        write!(
            f,
            "{} {} = ",
            SyntacticKind::Impl.get_syntax(),
            item_id.name(f.db),
        )?;
        Self::write_syntactic_evaluation(f, item_id)?;
        Ok(())
    }
}

impl SyntacticWriter for ModuleTypeAliasId {}

impl HirDisplay for ModuleTypeAliasId {
    fn hir_fmt(
        &self,
        f: &mut HirFormatter<'_>,
        item_id: DocumentableItemId,
    ) -> Result<(), fmt::Error> {
        let module_item_info = Self::get_module_item_info(item_id, f.db);
        f.write_visibility(module_item_info.visibility)?;
        write!(
            f,
            "{} {} = ",
            SyntacticKind::Impl.get_syntax(),
            item_id.name(f.db),
        )?;
        Self::write_syntactic_evaluation(f, item_id)?;
        Ok(())
    }
}

impl HirDisplay for TraitTypeId {
    fn hir_fmt(
        &self,
        f: &mut HirFormatter<'_>,
        item_id: DocumentableItemId,
    ) -> Result<(), fmt::Error> {
        write!(
            f,
            "{} {};",
            SyntacticKind::Type.get_syntax(),
            item_id.name(f.db)
        )?;
        Ok(())
    }
}

impl HirDisplay for ImplTypeDefId {
    fn hir_fmt(
        &self,
        f: &mut HirFormatter<'_>,
        item_id: DocumentableItemId,
    ) -> Result<(), fmt::Error> {
        let resolved_type = f.db.impl_type_def_resolved_type(*self).unwrap();
        write!(
            f,
            "{} {} = {};",
            SyntacticKind::Type.get_syntax(),
            item_id.name(f.db),
            extract_and_format(&resolved_type.format(f.db)),
        )?;
        Ok(())
    }
}

impl HirDisplay for ExternTypeId {
    fn hir_fmt(
        &self,
        f: &mut HirFormatter<'_>,
        item_id: DocumentableItemId,
    ) -> Result<(), fmt::Error> {
        let module_item_info = Self::get_module_item_info(item_id, f.db);
        f.write_visibility(module_item_info.visibility)?;
        write!(
            f,
            "{} {}",
            SyntacticKind::ExternType.get_syntax(),
            item_id.name(f.db),
        )?;
        let generic_params = f.db.extern_type_declaration_generic_params(*self).unwrap();
        if !generic_params.is_empty() {
            let mut count = generic_params.len();
            f.write_str("<")?;
            generic_params.iter().for_each(|param| {
                if count == 1 {
                    write!(f, "{}>;", param.id().name(f.db).unwrap()).unwrap();
                } else {
                    write!(f, "{}, ", param.id().name(f.db).unwrap()).unwrap();
                    count -= 1;
                }
            })
        };
        Ok(())
    }
}

impl FunctionSignatureWriter for ExternFunctionId {}

impl HirDisplay for ExternFunctionId {
    fn hir_fmt(
        &self,
        f: &mut HirFormatter<'_>,
        item_id: DocumentableItemId,
    ) -> Result<(), fmt::Error> {
        let module_item_info = Self::get_module_item_info(item_id, f.db);
        f.write_visibility(module_item_info.visibility)?;
        let signature = f.db.extern_function_signature(*self).unwrap();
        Self::write_function_signature(
            self,
            f,
            signature.clone(),
            item_id,
            SyntacticKind::ExternFunction,
        )?;
        if !signature.implicits.is_empty() {
            write!(f, " implicits(")?;

            let mut count = signature.implicits.len();
            signature.implicits.iter().for_each(|type_id| {
                write!(
                    f,
                    "{}{}",
                    type_id.format(f.db),
                    if count == 1 { ")" } else { ", " }
                )
                .unwrap();
                count -= 1;
            })
        }
        if !signature.panicable {
            write!(f, " nopanic")?;
        };
        write!(f, ";")?;
        Ok(())
    }
}

impl<'a> HirFormatter<'a> {
    pub fn write_visibility(&mut self, vis: Visibility) -> Result<(), fmt::Error> {
        match vis {
            Visibility::Public => write!(self, "pub "),
            Visibility::PublicInCrate => write!(self, "pub(crate) "),
            Visibility::Private => Ok(()),
        }
    }
}
