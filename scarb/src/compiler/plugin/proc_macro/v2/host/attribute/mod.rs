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
use crate::compiler::plugin::proc_macro::v2::{ProcMacroAuxData, ProcMacroId};
use cairo_lang_defs::plugin::{
    DynGeneratedFileAuxData, PluginDiagnostic, PluginGeneratedFile, PluginResult,
};
use cairo_lang_macro::AuxData;
use cairo_lang_syntax::node::db::SyntaxGroup;
use cairo_lang_syntax::node::ids::SyntaxStablePtrId;
pub use inner_attribute::*;
pub use item_attribute::*;
use smol_str::SmolStr;

#[derive(Default)]
pub struct AttributePluginResult {
    diagnostics: Vec<PluginDiagnostic>,
    remove_original_item: bool,
    code: Option<PluginGeneratedFile>,
}

impl AttributePluginResult {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_diagnostics(
        mut self,
        db: &dyn SyntaxGroup,
        call_site_stable_ptr: SyntaxStablePtrId,
        diagnostics: Vec<AdaptedDiagnostic>,
    ) -> Self {
        let diagnostics = diagnostics.into_iter().map(Into::into).collect();
        self.diagnostics = into_cairo_diagnostics(db, diagnostics, call_site_stable_ptr);
        self
    }

    pub fn with_plugin_diagnostics(mut self, diagnostics: Vec<PluginDiagnostic>) -> Self {
        self.diagnostics = diagnostics;
        self
    }

    pub fn with_remove_original_item(mut self, remove: bool) -> Self {
        self.remove_original_item = remove;
        self
    }

    pub fn with_code(
        mut self,
        name: SmolStr,
        content: String,
        code_mappings: Vec<AdaptedCodeMapping>,
        aux_data: Option<AuxData>,
        id: ProcMacroId,
    ) -> Self {
        let diagnostics_note = Some(format!(
            "this error originates in the attribute macro: `{}`",
            id.expansion.cairo_name
        ));
        let aux_data = aux_data.map(|new_aux_data| {
            DynGeneratedFileAuxData::new(EmittedAuxData::new(ProcMacroAuxData::new(
                new_aux_data.into(),
                id.clone(),
            )))
        });
        self.code = Some(PluginGeneratedFile {
            name,
            content,
            code_mappings: code_mappings.into_iter().map(Into::into).collect(),
            aux_data,
            diagnostics_note,
        });
        self
    }

    pub fn with_plugin_generated_file(mut self, generated_file: PluginGeneratedFile) -> Self {
        self.code = Some(generated_file);
        self
    }
}

impl From<AttributePluginResult> for PluginResult {
    fn from(value: AttributePluginResult) -> Self {
        PluginResult {
            diagnostics: value.diagnostics,
            remove_original_item: value.remove_original_item,
            code: value.code,
        }
    }
}
