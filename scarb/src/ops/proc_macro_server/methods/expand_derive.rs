use std::hash::{Hash, Hasher};
use std::sync::{Arc, Mutex};

use super::{Handler, interface_code_mapping_from_cairo};
use crate::compiler::plugin::proc_macro::ExpansionKind;
use crate::compiler::plugin::proc_macro::v2::derive::generate_code_mappings_with_offset;
use crate::compiler::plugin::proc_macro::{
    Expansion, ExpansionQuery, ProcMacroApiVersion, ProcMacroInstance,
};
use crate::core::Config;
use crate::ops::store::ProcMacroStore;
use anyhow::{Context, Result};
use cairo_lang_filesystem::span::TextWidth;
use cairo_lang_macro::{TextSpan, TokenStream as TokenStreamV2};
use cairo_lang_macro_v1::TokenStream as TokenStreamV1;
use scarb_proc_macro_server_types::conversions::{diagnostic_v1_to_v2, token_stream_v2_to_v1};
use scarb_proc_macro_server_types::methods::{
    CodeMapping, CodeOrigin, ProcMacroResult, expand::ExpandDerive,
};
use scarb_stable_hash::StableHasher;

impl Handler for ExpandDerive {
    fn handle(
        _config: &Config,
        proc_macros: Arc<Mutex<ProcMacroStore>>,
        params: Self::Params,
    ) -> Result<Self::Response> {
        let Self::Params {
            context,
            mut derives,
            item,
            call_site,
        } = params;

        // We need derives to be in deterministic order for hasing later.
        // LS capable of using fingerprint hash sends already sorted derives, so this should be linear.
        derives.sort();

        let mut derived_code = String::new();
        let mut all_diagnostics = vec![];
        let mut code_mappings = vec![];
        // Needed to provide offset for code mappings in v2-style macros
        let mut current_width = TextWidth::default();
        let mut hasher = StableHasher::new();

        for derive in derives {
            let expansion =
                ExpansionQuery::with_expansion_name(derive.clone(), ExpansionKind::Derive);

            let (proc_macro_instance, hash) = proc_macros
                .lock()
                .unwrap()
                .get_instance_and_hash(&context, &expansion)
                .with_context(|| {
                    format!("No \"{derive}\" derive macros found in scope {context:?}")
                })?;

            let expansion = proc_macro_instance
                .find_expansion(&expansion)
                .with_context(|| {
                    format!("No \"{derive}\" derive macros found in scope {context:?}")
                })?;

            let result = match proc_macro_instance.api_version() {
                ProcMacroApiVersion::V1 => expand_derive_v1(
                    &proc_macro_instance,
                    hash,
                    current_width,
                    call_site.clone(),
                    expansion,
                    token_stream_v2_to_v1(&item),
                ),
                ProcMacroApiVersion::V2 => expand_derive_v2(
                    &proc_macro_instance,
                    hash,
                    current_width,
                    expansion,
                    call_site.clone(),
                    item.clone(),
                ),
            }?;

            result.fingerprint.hash(&mut hasher);

            current_width = current_width + TextWidth::from_str(&result.token_stream.to_string());

            if result.code_mappings.is_some() {
                code_mappings.extend(result.code_mappings.unwrap());
            }

            // Register diagnostics.
            all_diagnostics.extend(result.diagnostics);
            // Add generated code.
            derived_code.push_str(&result.token_stream.to_string());
        }

        Ok(ProcMacroResult {
            token_stream: TokenStreamV1::new(derived_code),
            diagnostics: all_diagnostics,
            code_mappings: Some(code_mappings),
            fingerprint: hasher.finish(),
        })
    }
}

fn expand_derive_v1(
    proc_macro_instance: &Arc<ProcMacroInstance>,
    fingerprint: u64,
    current_width: TextWidth,
    call_site: TextSpan,
    expansion: &Expansion,
    item: TokenStreamV1,
) -> Result<ProcMacroResult> {
    let result = proc_macro_instance.try_v1()?.generate_code(
        expansion.expansion_name.clone(),
        TokenStreamV1::empty(),
        item,
    );

    // Default mapping for v1 derives
    let added_length = TextWidth::from_str(&result.token_stream.to_string());
    let code_mappings = Some(vec![CodeMapping {
        span: TextSpan {
            start: current_width.as_u32(),
            end: (current_width + added_length).as_u32(),
        },
        origin: CodeOrigin::Span(call_site.clone()),
    }]);

    Ok(ProcMacroResult {
        token_stream: result.token_stream,
        diagnostics: result.diagnostics.iter().map(diagnostic_v1_to_v2).collect(),
        code_mappings,
        fingerprint,
    })
}

fn expand_derive_v2(
    proc_macro_instance: &Arc<ProcMacroInstance>,
    fingerprint: u64,
    current_width: TextWidth,
    expansion: &Expansion,
    call_site: TextSpan,
    item: TokenStreamV2,
) -> Result<ProcMacroResult> {
    let result = proc_macro_instance.try_v2()?.generate_code(
        expansion.expansion_name.clone(),
        call_site.clone(),
        TokenStreamV2::empty(),
        item.clone(),
    );

    let code_mappings =
        generate_code_mappings_with_offset(&result.token_stream, call_site.clone(), current_width);

    Ok(ProcMacroResult {
        token_stream: token_stream_v2_to_v1(&result.token_stream),
        diagnostics: result.diagnostics,
        code_mappings: Some(
            code_mappings
                .into_iter()
                .map(interface_code_mapping_from_cairo)
                .collect(),
        ),
        fingerprint,
    })
}
