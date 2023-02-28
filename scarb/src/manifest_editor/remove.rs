use anyhow::{anyhow, Result};
use toml_edit::Document;

use crate::core::PackageName;
use crate::ui::Status;

use super::tomlx::get_table_mut;
use super::{Op, OpCtx};

#[derive(Debug, Default)]
pub struct RemoveDependency {
    pub dep: PackageName,
}

impl Op for RemoveDependency {
    #[tracing::instrument(level = "trace", skip(doc, ctx))]
    fn apply_to(self: Box<Self>, doc: &mut Document, ctx: OpCtx<'_>) -> Result<()> {
        let tab = get_table_mut(doc, &["dependencies"])?;

        let dep_key = self
            .dep
            .ok_or_else(|| anyhow!("please specify package name"))?;

        // section is hardcoded as there's no support for other section types yet
        ctx.opts.config.ui().print(Status::new(
            "Removing",
            &format!("{dep_key} from dependencies"),
        ));

        tab.as_table_like_mut()
            .unwrap()
            .remove(dep_key.as_str())
            .ok_or_else(|| {
                anyhow!("the dependency `{dep_key}` could not be found in `dependencies`")
            })?;

        Ok(())
    }
}
