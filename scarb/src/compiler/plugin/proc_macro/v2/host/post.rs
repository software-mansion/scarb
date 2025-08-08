use crate::compiler::plugin::proc_macro::FULL_PATH_MARKER_KEY;
use crate::compiler::plugin::proc_macro::v2::ProcMacroHostPlugin;
use crate::core::PackageId;
use anyhow::Result;
use cairo_lang_defs::ids::{ModuleItemId, TopLevelLanguageElementId};
use cairo_lang_diagnostics::ToOption;
use cairo_lang_macro::FullPathMarker;
use cairo_lang_semantic::db::SemanticGroup;
use cairo_lang_semantic::items::attribute::SemanticQueryAttrs;
use cairo_lang_syntax::attribute::structured::{Attribute, AttributeArgVariant};
use cairo_lang_syntax::node::ast::Expr;
use itertools::Itertools;
use std::collections::HashMap;
use tracing::{debug, trace_span};

impl ProcMacroHostPlugin {
    #[tracing::instrument(level = "trace", skip_all)]
    pub fn post_process(&self, db: &dyn SemanticGroup) -> Result<()> {
        let aux_data = self.collect_aux_data(db);
        // Only collect full path markers if any have been registered by macros.
        let any_markers = !self
            .full_path_markers
            .read()
            .unwrap()
            .values()
            .all(|v| v.is_empty());
        let markers = if any_markers {
            self.collect_full_path_markers(db)
        } else {
            Default::default()
        };
        for instance in self.instances.iter() {
            let _ = trace_span!(
                "post_process_callback",
                instance = %instance.package_id()
            )
            .entered();
            let instance_markers = if any_markers {
                {
                    self.full_path_markers
                        .read()
                        .unwrap()
                        .get(&instance.package_id())
                        .cloned()
                        .unwrap_or_default()
                }
            } else {
                Default::default()
            };
            let markers_for_instance = markers
                .iter()
                .filter(|(key, _)| instance_markers.contains(key))
                .map(|(key, full_path)| FullPathMarker {
                    key: key.clone(),
                    full_path: full_path.clone(),
                })
                .collect_vec();
            let data = aux_data
                .get(&instance.package_id())
                .cloned()
                .unwrap_or_default();
            debug!("calling post processing callback with: {data:?}");
            instance
                .try_v2()
                .expect("procedural macro using v1 api used in a context expecting v2 api")
                .post_process_callback(data.clone(), markers_for_instance);
        }
        Ok(())
    }

    #[tracing::instrument(level = "trace", skip_all)]
    fn collect_full_path_markers(&self, db: &dyn SemanticGroup) -> HashMap<String, String> {
        let mut markers: HashMap<String, String> = HashMap::new();
        for crate_id in db.crates() {
            let modules = db.crate_modules(crate_id);
            for module_id in modules.iter() {
                let Ok(module_items) = db.module_items(*module_id) else {
                    continue;
                };
                for item_id in module_items.iter() {
                    let attr = match item_id {
                        ModuleItemId::Struct(id) => {
                            id.query_attr(db, FULL_PATH_MARKER_KEY).to_option()
                        }
                        ModuleItemId::Enum(id) => {
                            id.query_attr(db, FULL_PATH_MARKER_KEY).to_option()
                        }
                        ModuleItemId::FreeFunction(id) => {
                            id.query_attr(db, FULL_PATH_MARKER_KEY).to_option()
                        }
                        _ => None,
                    };

                    let keys = attr
                        .unwrap_or_default()
                        .into_iter()
                        .filter_map(|attr| Self::extract_key(db, attr))
                        .collect_vec();
                    let full_path = item_id.full_path(db.upcast());
                    for key in keys {
                        markers.insert(key, full_path.clone());
                    }
                }
            }
        }
        markers
    }

    fn extract_key<'db>(db: &'db dyn SemanticGroup, attr: Attribute<'db>) -> Option<String> {
        if attr.id != FULL_PATH_MARKER_KEY {
            return None;
        }

        for arg in attr.args.clone() {
            if let AttributeArgVariant::Unnamed(Expr::String(s)) = arg.variant {
                return s.string_value(db.upcast());
            }
        }

        None
    }

    pub(crate) fn register_full_path_markers(&self, package_id: PackageId, markers: Vec<String>) {
        self.full_path_markers
            .write()
            .unwrap()
            .entry(package_id)
            .and_modify(|markers| markers.extend(markers.clone()))
            .or_insert(markers);
    }
}
