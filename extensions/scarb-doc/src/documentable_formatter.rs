use crate::bounding::{
    end_bounding, extract_and_format, get_postfix, get_syntactic_mutability,
    get_syntactic_visibility, start_bounding, BoundingPostfix, BoundingType, SyntacticKind,
};
use crate::db::ScarbDocDatabase;
use cairo_lang_defs::ids::{
    ConstantId, EnumId, FreeFunctionId, ImplAliasId, ImplConstantDefId, ImplDefId, ImplFunctionId,
    ImplTypeDefId, StructId, TraitConstantId, TraitFunctionId, TraitId, TraitImplId, TraitTypeId,
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
use cairo_lang_utils::Upcast;
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
    pub buf: String,
    pub members_buff: Option<HashMap<SmolStr, String>>,
}

impl fmt::Write for HirFormatter<'_> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.buf.push_str(s);
        Ok(())
    }
}

impl HirDisplay for EnumId {
    fn hir_fmt(
        &self,
        f: &mut HirFormatter<'_>,
        item_id: DocumentableItemId,
    ) -> Result<(), fmt::Error> {
        let module_item_info = Self::get_module_item_info(item_id, f.db);
        write_visibility(module_item_info.visibility, f)?;
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
        write_visibility(module_item_info.visibility, f)?;

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
    ) -> Result<(), fmt::Error> {
        write!(
            f,
            "{} {}{}",
            SyntacticKind::Function.get_syntax(),
            item_id.name(f.db),
            start_bounding(BoundingType::Parenthesis)
        )?;

        let mut count = signature.params.len();
        let mut postfix = ", ";
        signature.params.iter().for_each(|param| {
            if count == 1 {
                postfix = "";
            }
            write!(
                f,
                "{}{}: {}{}",
                get_syntactic_mutability(&param.mutability),
                param.name,
                extract_and_format(&param.ty.format(f.db)),
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
}

impl FunctionSignatureWriter for FreeFunctionId {}

impl HirDisplay for FreeFunctionId {
    fn hir_fmt(
        &self,
        f: &mut HirFormatter<'_>,
        item_id: DocumentableItemId,
    ) -> Result<(), fmt::Error> {
        let module_item_info = Self::get_module_item_info(item_id, f.db);

        write_visibility(module_item_info.visibility, f)?;
        let signature = f.db.free_function_signature(*self).unwrap();
        Self::write_function_signature(self, f, signature, item_id)
    }
}

fn write_syntactic_constant_evaluation(
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

impl HirDisplay for ConstantId {
    fn hir_fmt(
        &self,
        f: &mut HirFormatter<'_>,
        item_id: DocumentableItemId,
    ) -> Result<(), fmt::Error> {
        let module_item_info = Self::get_module_item_info(item_id, f.db);
        write_visibility(module_item_info.visibility, f)?;
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
                    write_syntactic_constant_evaluation(f, item_id)?;
                    write!(f.buf, " // = {}", value)?;
                }
            }
            _ => {
                write_syntactic_constant_evaluation(f, item_id)?;
            }
        };
        Ok(())
    }
}

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
        write_syntactic_constant_evaluation(f, item_id)?;
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
        Self::write_function_signature(self, f, signature, item_id)
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
        Self::write_function_signature(self, f, signature, item_id)
    }
}

impl HirDisplay for TraitId {
    fn hir_fmt(
        &self,
        f: &mut HirFormatter<'_>,
        item_id: DocumentableItemId,
    ) -> Result<(), fmt::Error> {
        let module_item_info = Self::get_module_item_info(item_id, f.db);
        write_visibility(module_item_info.visibility, f)?;

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
        write_visibility(module_item_info.visibility, f)?;
        let concrete_trait_id = f.db.impl_def_concrete_trait(*self).unwrap();
        write!(
            f,
            "{} {} of {}",
            SyntacticKind::Impl.get_syntax(),
            item_id.name(f.db),
            concrete_trait_id.full_path(f.db), // todo: format
        )?;
        Ok(())
    }
}

impl HirDisplay for ImplAliasId {
    fn hir_fmt(
        &self,
        f: &mut HirFormatter<'_>,
        item_id: DocumentableItemId,
    ) -> Result<(), fmt::Error> {
        let module_item_info = Self::get_module_item_info(item_id, f.db);
        write_visibility(module_item_info.visibility, f)?;
        // todo fixme
        write!(
            f,
            "{} {} = ...",
            SyntacticKind::Impl.get_syntax(),
            item_id.name(f.db),
        )?;

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

impl HirDisplay for TraitImplId {
    fn hir_fmt(
        &self,
        _f: &mut HirFormatter<'_>,
        _item_id: DocumentableItemId,
    ) -> Result<(), fmt::Error> {
        // todo
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

pub fn write_visibility(vis: Visibility, f: &mut HirFormatter<'_>) -> Result<(), fmt::Error> {
    match vis {
        Visibility::Public => {
            write!(f, "pub ")
        }
        Visibility::PublicInCrate => {
            write!(f, "pub(crate) ")
        }
        Visibility::Private => {
            write!(f, "")
        }
    }
}
