use std::sync::{Arc, Mutex};

use anyhow::Result;
use scarb_proc_macro_server_types::methods::Method;

use crate::core::Config;
use crate::ops::store::ProcMacroStore;

pub mod defined_macros;
pub mod expand_attribute;
pub mod expand_derive;
pub mod expand_inline;

pub trait Handler: Method {
    fn handle(
        config: &Config,
        proc_macros: Arc<Mutex<ProcMacroStore>>,
        params: Self::Params,
    ) -> Result<Self::Response>;
}

use cairo_lang_filesystem::ids::{CodeMapping as CairoCodeMapping, CodeOrigin as CairoCodeOrigin};
use cairo_lang_filesystem::span::{TextOffset as CairoTextOffset, TextSpan as CairoTextSpan};
use scarb_proc_macro_server_types::methods::{
    CodeMapping as InterfaceCodeMapping, CodeOrigin as InterfaceCodeOrigin,
    TextOffset as InterfaceTextOffset, TextSpan as InterfaceTextSpan,
};

fn interface_text_offset_from_cairo(cairo_text_offset: CairoTextOffset) -> InterfaceTextOffset {
    InterfaceTextOffset::default() + cairo_text_offset.as_u32()
}

fn interface_text_span_from_cairo(cairo_text_span: CairoTextSpan) -> InterfaceTextSpan {
    InterfaceTextSpan {
        start: interface_text_offset_from_cairo(cairo_text_span.start),
        end: interface_text_offset_from_cairo(cairo_text_span.end),
    }
}

pub fn interface_code_mapping_from_cairo(cairo_mapping: CairoCodeMapping) -> InterfaceCodeMapping {
    InterfaceCodeMapping {
        span: interface_text_span_from_cairo(cairo_mapping.span),
        origin: match cairo_mapping.origin {
            CairoCodeOrigin::Start(offset) => {
                InterfaceCodeOrigin::Start(interface_text_offset_from_cairo(offset))
            }
            CairoCodeOrigin::Span(span) => {
                InterfaceCodeOrigin::Span(interface_text_span_from_cairo(span))
            }
            CairoCodeOrigin::CallSite(span) => {
                InterfaceCodeOrigin::CallSite(interface_text_span_from_cairo(span))
            }
        },
    }
}
