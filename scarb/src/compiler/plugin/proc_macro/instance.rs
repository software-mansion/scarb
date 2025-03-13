use crate::compiler::plugin::CairoPluginProps;
use crate::core::{Package, TargetKind};
use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(
    Debug, Clone, Copy, Serialize, Deserialize, Default, PartialEq, Eq, PartialOrd, Ord, Hash,
)]
#[serde(rename_all = "lowercase")]
pub enum ProcMacroApiVersion {
    #[default]
    V1,
    V2,
}

pub trait ProcMacroApiVersionReader {
    fn api_version(&self) -> Result<ProcMacroApiVersion>;
}

impl ProcMacroApiVersionReader for Package {
    fn api_version(&self) -> Result<ProcMacroApiVersion> {
        assert!(self.is_cairo_plugin());
        let target = self.fetch_target(&TargetKind::CAIRO_PLUGIN)?;
        let props: CairoPluginProps = target.props()?;
        Ok(props.api)
    }
}
