mod child_nodes;
mod inner_attribute;
mod item_attribute;
mod parse_attributes;
mod span_adapter;

use crate::compiler::plugin::proc_macro::v2::host::attribute::span_adapter::{
    AdaptedCodeMapping, AdaptedDiagnostic,
};
use crate::compiler::plugin::proc_macro::v2::host::aux_data::EmittedAuxData;
use crate::compiler::plugin::proc_macro::v2::host::conversion::into_cairo_diagnostics;
use cairo_lang_defs::patcher::PatchBuilder;
use cairo_lang_defs::plugin::{
    DynGeneratedFileAuxData, PluginDiagnostic, PluginGeneratedFile, PluginResult,
};
use cairo_lang_filesystem::ids::CodeMapping;
use cairo_lang_syntax::node::ids::SyntaxStablePtrId;
pub use inner_attribute::*;
pub use item_attribute::*;
use salsa::Database;

#[derive(Default)]
pub struct AttributePluginResult<'db> {
    diagnostics: Vec<PluginDiagnostic<'db>>,
    remove_original_item: bool,
    code: Option<PluginGeneratedFile>,
}

impl<'db> AttributePluginResult<'db> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_diagnostics(
        mut self,
        db: &'db dyn Database,
        call_site_stable_ptr: SyntaxStablePtrId<'db>,
        diagnostics: Vec<AdaptedDiagnostic>,
    ) -> Self {
        let diagnostics = diagnostics.into_iter().map(Into::into).collect();
        self.diagnostics = into_cairo_diagnostics(db, diagnostics, call_site_stable_ptr);
        self
    }

    pub fn with_plugin_diagnostics(mut self, diagnostics: Vec<PluginDiagnostic<'db>>) -> Self {
        self.diagnostics = diagnostics;
        self
    }

    pub fn with_remove_original_item(mut self, remove: bool) -> Self {
        self.remove_original_item = remove;
        self
    }

    pub fn with_generated_file(mut self, generated_file: AttributeGeneratedFile) -> Self {
        self.code = Some(generated_file.into());
        self
    }
}

impl<'db> From<AttributePluginResult<'db>> for PluginResult<'db> {
    fn from(value: AttributePluginResult<'db>) -> Self {
        PluginResult {
            diagnostics: value.diagnostics,
            remove_original_item: value.remove_original_item,
            code: value.code,
        }
    }
}

pub struct AttributeGeneratedFile {
    name: String,
    content: String,
    code_mappings: Vec<CodeMapping>,
    aux_data: Option<DynGeneratedFileAuxData>,
    diagnostics_note: Option<String>,
}

impl AttributeGeneratedFile {
    pub fn new(name: impl ToString) -> Self {
        Self {
            name: name.to_string(),
            content: Default::default(),
            code_mappings: Default::default(),
            aux_data: Default::default(),
            diagnostics_note: Default::default(),
        }
    }

    pub fn from_patch_builder(name: impl ToString, item_builder: PatchBuilder<'_>) -> Self {
        let (expanded, mut code_mappings) = item_builder.build();
        // PatchBuilder::build() adds additional mapping at the end,
        // which wraps the whole outputted code.
        // We remove it, so we can properly translate locations spanning multiple token spans.
        code_mappings.pop();
        Self {
            name: name.to_string(),
            content: expanded,
            code_mappings,
            aux_data: Default::default(),
            diagnostics_note: Default::default(),
        }
    }

    pub fn with_content(mut self, content: impl ToString) -> Self {
        self.content = content.to_string();
        self
    }

    pub fn with_code_mappings(mut self, code_mappings: Vec<AdaptedCodeMapping>) -> Self {
        self.code_mappings = code_mappings.into_iter().map(Into::into).collect();
        self
    }

    pub fn with_aux_data(mut self, aux_data: EmittedAuxData) -> Self {
        if aux_data.is_empty() {
            self.aux_data = None;
        } else {
            self.aux_data = Some(DynGeneratedFileAuxData::new(aux_data));
        }
        self
    }

    pub fn with_diagnostics_note(mut self, diagnostics_note: impl ToString) -> Self {
        self.diagnostics_note = Some(diagnostics_note.to_string());
        self
    }
}

impl From<AttributeGeneratedFile> for PluginGeneratedFile {
    fn from(value: AttributeGeneratedFile) -> Self {
        PluginGeneratedFile {
            name: value.name,
            content: value.content,
            code_mappings: value.code_mappings,
            aux_data: value.aux_data,
            diagnostics_note: value.diagnostics_note,
            is_unhygienic: false,
        }
    }
}
