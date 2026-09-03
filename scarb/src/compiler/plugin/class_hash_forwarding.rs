//! Generates, for every `#[starknet::contract]`, a sibling module exposing its own class hash as
//! a compile-time constant, so other contracts can embed it via `starknet::ForwardingClassHash`
//! (static forwarding). Scarb-owned codegen; resolution happens in `starknet_contract::forwarding`.

use cairo_lang_defs::plugin::{
    MacroPlugin, MacroPluginMetadata, PluginGeneratedFile, PluginResult,
};
use cairo_lang_filesystem::ids::SmolStrId;
use cairo_lang_semantic::plugin::PluginSuite;
use cairo_lang_syntax::node::helpers::QueryAttrs;
use cairo_lang_syntax::node::{Terminal, ast};
use salsa::Database;

/// Attribute Cairo's Starknet plugin uses to mark a contract module (stable, kept as a literal).
const CONTRACT_ATTR: &str = "starknet::contract";

/// Suffix naming a contract's generated class-hash sibling, e.g. `counter_contract__class_hash__`.
/// A sibling, not a nested submodule: generated code attaches to the module *containing* the
/// declaration that triggered it, not inside the declared module itself.
pub const CLASS_HASH_MODULE_SUFFIX: &str = "__class_hash__";

/// Must match `cairo_lang_sierra_generator::db::EXTERNALLY_PROVIDED_CONST`.
const EXTERNALLY_PROVIDED_CONST: &str = "__externally_provided_const__";

/// Plugin suite generating the class-hash sibling module for every Starknet contract.
pub fn class_hash_forwarding_plugin_suite() -> PluginSuite {
    let mut suite = PluginSuite::default();
    suite.add_plugin::<ClassHashForwardingPlugin>();
    suite
}

#[derive(Debug, Default)]
#[non_exhaustive]
pub struct ClassHashForwardingPlugin;

impl MacroPlugin for ClassHashForwardingPlugin {
    fn generate_code<'db>(
        &self,
        db: &'db dyn Database,
        item_ast: ast::ModuleItem<'db>,
        _metadata: &MacroPluginMetadata<'_>,
    ) -> PluginResult<'db> {
        let ast::ModuleItem::Module(item_module) = &item_ast else {
            return PluginResult::default();
        };
        if !item_module.has_attr(db, CONTRACT_ATTR) {
            return PluginResult::default();
        }
        let contract_name = item_module.name(db).text(db).long(db);
        let module_name = format!("{contract_name}{CLASS_HASH_MODULE_SUFFIX}");
        let content = format!(
            "#[doc(hidden)]\npub mod {module_name} {{\n\
             \x20   #[allow(extern_outside_corelib)]\n\
             \x20   extern fn {EXTERNALLY_PROVIDED_CONST}() -> starknet::ClassHash nopanic;\n\n\
             \x20   pub fn class_hash() -> starknet::ClassHash {{\n\
             \x20       {EXTERNALLY_PROVIDED_CONST}()\n\
             \x20   }}\n\n\
             \x20   #[feature(\"forward-impl\")]\n\
             \x20   pub impl ForwardingClassHashImpl<T> of starknet::ForwardingClassHash<T> {{\n\
             \x20       fn class_hash(self: @T) -> starknet::ClassHash {{\n\
             \x20           class_hash()\n\
             \x20       }}\n\
             \x20   }}\n\
             }}\n"
        );
        PluginResult {
            code: Some(PluginGeneratedFile {
                name: format!("{contract_name}_class_hash"),
                content,
                code_mappings: Default::default(),
                aux_data: None,
                diagnostics_note: Default::default(),
                is_unhygienic: false,
            }),
            diagnostics: vec![],
            remove_original_item: false,
        }
    }

    fn declared_attributes<'db>(&self, db: &'db dyn Database) -> Vec<SmolStrId<'db>> {
        vec![SmolStrId::from(db, CONTRACT_ATTR)]
    }
}
