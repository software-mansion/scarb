use anyhow::{anyhow, Result};
use toml_edit::Document;

use scarb_ui::components::Status;

use crate::core::PackageName;
use crate::manifest_editor::DepType;

use super::tomlx::get_table_mut;
use super::{Op, OpCtx};

#[derive(Debug)]
pub struct RemoveDependency {
    pub dep: PackageName,
    pub dep_type: DepType,
}

impl Op for RemoveDependency {
    #[tracing::instrument(level = "trace", skip(doc, ctx))]
    fn apply_to(self: Box<Self>, doc: &mut Document, ctx: OpCtx<'_>) -> Result<()> {
        let tab = get_table_mut(doc, &[self.dep_type.toml_section_str()])?;

        // section is hardcoded as there's no support for other section types yet
        ctx.opts.config.ui().print(Status::new(
            "Removing",
            &format!("{} from {}", self.dep, self.dep_type.toml_section_str()),
        ));

        tab.as_table_like_mut()
            .unwrap()
            .remove(self.dep.as_str())
            .ok_or_else(|| {
                anyhow!(
                    "the dependency `{}` could not be found in `{}`",
                    self.dep,
                    self.dep_type.toml_section_str(),
                )
            })?;

        Ok(())
    }
}
