use cairo_lang_defs::ids::{GenericTypeId, LookupItemId, ModuleId, ModuleItemId, TraitItemId};
use cairo_lang_diagnostics::DiagnosticsBuilder;
use cairo_lang_doc::documentable_item::DocumentableItemId;
use cairo_lang_doc::parser::CommentLinkToken;
use cairo_lang_filesystem::db::FilesGroup;
use cairo_lang_filesystem::ids::{FileKind, FileLongId, SmolStrId, VirtualFile};
use cairo_lang_parser::parser::Parser;
use cairo_lang_semantic::diagnostic::{NotFoundItemType, SemanticDiagnostics};
use cairo_lang_semantic::expr::inference::InferenceId;
use cairo_lang_semantic::items::functions::GenericFunctionId;
use cairo_lang_semantic::lsp_helpers::LspHelpers;
use cairo_lang_semantic::resolve::{AsSegments, ResolutionContext, ResolvedGenericItem, Resolver};
use cairo_lang_syntax::node::ast::{Expr, ExprPath};
use cairo_lang_utils::Intern;

use crate::db::ScarbDocDatabase;

pub fn resolve_linked_item<'db>(
    db: &'db ScarbDocDatabase,
    item_id: DocumentableItemId<'db>,
    link: &CommentLinkToken,
) -> Option<DocumentableItemId<'db>> {
    let path = link.md_link.dest_text.as_deref()?;
    resolve_linked_item_from_path(db, item_id, path)
}

fn resolve_linked_item_from_path<'db>(
    db: &'db ScarbDocDatabase,
    item_id: DocumentableItemId<'db>,
    path: &str,
) -> Option<DocumentableItemId<'db>> {
    let syntax_node = item_id.stable_location(db)?.syntax_node(db);
    let containing_module = db.find_module_containing_node(syntax_node)?;
    let mut resolver = Resolver::new(db, containing_module, InferenceId::NoContext);
    let mut diagnostics = SemanticDiagnostics::new(containing_module);
    let segments = parse_comment_link_path(db, path)?;
    resolver
        .resolve_generic_path(
            &mut diagnostics,
            segments.to_segments(db),
            NotFoundItemType::Identifier,
            ResolutionContext::Default,
        )
        .ok()
        .and_then(|resolved| resolved.to_documentable_item_id(db))
}

fn parse_comment_link_path<'db>(db: &'db ScarbDocDatabase, path: &str) -> Option<ExprPath<'db>> {
    let virtual_file = FileLongId::Virtual(VirtualFile {
        parent: None,
        name: SmolStrId::from(db, "doc_link"),
        content: SmolStrId::from(db, path),
        code_mappings: [].into(),
        kind: FileKind::Module,
        original_item_removed: false,
    })
    .intern(db);

    let content = db.file_content(virtual_file)?;
    let expr = Parser::parse_file_expr(
        db,
        &mut DiagnosticsBuilder::default(),
        virtual_file,
        content,
    );
    if let Expr::Path(expr_path) = expr {
        Some(expr_path)
    } else {
        None
    }
}

trait ToDocumentableItemId<'db> {
    fn to_documentable_item_id(self, db: &'db ScarbDocDatabase) -> Option<DocumentableItemId<'db>>;
}

impl<'db> ToDocumentableItemId<'db> for ResolvedGenericItem<'db> {
    /// Converts the [ResolvedGenericItem] to [DocumentableItemId].
    /// Returns None only for a Variable, as those are not a supported documentable item.
    fn to_documentable_item_id(self, db: &'db ScarbDocDatabase) -> Option<DocumentableItemId<'db>> {
        match self {
            ResolvedGenericItem::GenericConstant(id) => Some(DocumentableItemId::LookupItem(
                LookupItemId::ModuleItem(ModuleItemId::Constant(id)),
            )),
            ResolvedGenericItem::Module(ModuleId::Submodule(id)) => {
                Some(DocumentableItemId::LookupItem(LookupItemId::ModuleItem(
                    ModuleItemId::Submodule(id),
                )))
            }
            ResolvedGenericItem::Module(ModuleId::CrateRoot(id)) => {
                Some(DocumentableItemId::Crate(id))
            }
            ResolvedGenericItem::Module(ModuleId::MacroCall { .. }) => None,
            ResolvedGenericItem::GenericFunction(GenericFunctionId::Free(id)) => {
                Some(DocumentableItemId::LookupItem(LookupItemId::ModuleItem(
                    ModuleItemId::FreeFunction(id),
                )))
            }
            ResolvedGenericItem::GenericFunction(GenericFunctionId::Extern(id)) => {
                Some(DocumentableItemId::LookupItem(LookupItemId::ModuleItem(
                    ModuleItemId::ExternFunction(id),
                )))
            }
            ResolvedGenericItem::GenericFunction(GenericFunctionId::Impl(generic_impl_func)) => {
                if let Some(impl_function) = generic_impl_func.impl_function(db).ok().flatten() {
                    Some(DocumentableItemId::LookupItem(LookupItemId::ImplItem(
                        cairo_lang_defs::ids::ImplItemId::Function(impl_function),
                    )))
                } else {
                    Some(DocumentableItemId::LookupItem(LookupItemId::TraitItem(
                        TraitItemId::Function(generic_impl_func.function),
                    )))
                }
            }
            ResolvedGenericItem::GenericType(GenericTypeId::Struct(id)) => Some(
                DocumentableItemId::LookupItem(LookupItemId::ModuleItem(ModuleItemId::Struct(id))),
            ),
            ResolvedGenericItem::GenericType(GenericTypeId::Enum(id)) => Some(
                DocumentableItemId::LookupItem(LookupItemId::ModuleItem(ModuleItemId::Enum(id))),
            ),
            ResolvedGenericItem::GenericType(GenericTypeId::Extern(id)) => {
                Some(DocumentableItemId::LookupItem(LookupItemId::ModuleItem(
                    ModuleItemId::ExternType(id),
                )))
            }
            ResolvedGenericItem::GenericTypeAlias(id) => Some(DocumentableItemId::LookupItem(
                LookupItemId::ModuleItem(ModuleItemId::TypeAlias(id)),
            )),
            ResolvedGenericItem::GenericImplAlias(id) => Some(DocumentableItemId::LookupItem(
                LookupItemId::ModuleItem(ModuleItemId::ImplAlias(id)),
            )),
            ResolvedGenericItem::Trait(id) => Some(DocumentableItemId::LookupItem(
                LookupItemId::ModuleItem(ModuleItemId::Trait(id)),
            )),
            ResolvedGenericItem::Impl(id) => Some(DocumentableItemId::LookupItem(
                LookupItemId::ModuleItem(ModuleItemId::Impl(id)),
            )),
            ResolvedGenericItem::Macro(id) => Some(DocumentableItemId::LookupItem(
                LookupItemId::ModuleItem(ModuleItemId::MacroDeclaration(id)),
            )),
            ResolvedGenericItem::Variant(variant) => Some(DocumentableItemId::Variant(variant.id)),
            ResolvedGenericItem::TraitItem(id) => {
                Some(DocumentableItemId::LookupItem(LookupItemId::TraitItem(id)))
            }
            ResolvedGenericItem::Variable(_) => None,
        }
    }
}
