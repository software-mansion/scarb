use anyhow::Result;
use cairo_lang_compiler::db::RootDatabase;
use cairo_lang_defs::ids::ModuleId;
use cairo_lang_filesystem::ids::{BlobId, BlobLongId, FileId};
use cairo_lang_lowering::db::LoweringGroup;
use cairo_lang_utils::Upcast;
use cairo_lint::LinterAnalysisDatabase;

/// An abstract to inject virtual methods into the `LinterAnalysisDatabase` and `RootDatabase`
pub trait VirtualDatabaseWrapper {
    fn module_main_file(&self, module_id: ModuleId<'_>) -> Result<FileId<'_>>;
    fn intern_blob(&self, blob_id: BlobLongId) -> BlobId<'_>;
    fn as_lowering_group(&self) -> &dyn LoweringGroup;
}

impl VirtualDatabaseWrapper for LinterAnalysisDatabase {
    #[expect(unconditional_recursion)]
    fn module_main_file(&self, _module_id: ModuleId<'_>) -> Result<FileId<'_>> {
        self.module_main_file(_module_id)
    }
    #[expect(unconditional_recursion)]
    fn intern_blob(&self, _blob_id: BlobLongId) -> BlobId<'_> {
        self.intern_blob(_blob_id)
    }
    fn as_lowering_group(&self) -> &dyn LoweringGroup {
        self.upcast()
    }
}

impl VirtualDatabaseWrapper for RootDatabase {
    #[expect(unconditional_recursion)]
    fn module_main_file(&self, _module_id: ModuleId<'_>) -> Result<FileId<'_>> {
        self.module_main_file(_module_id)
    }
    #[expect(unconditional_recursion)]
    fn intern_blob(&self, _blob_id: BlobLongId) -> BlobId<'_> {
        self.intern_blob(_blob_id)
    }
    fn as_lowering_group(&self) -> &dyn LoweringGroup {
        self.upcast()
    }
}
