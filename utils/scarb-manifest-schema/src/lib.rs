//! Dynamic JSON Schema for Scarb manifest.
//!
//! Provides a way to generate and traverse the Scarb manifest JSON Schema and retrieve the
//! definition for a specific TOML path, such as `package.dependencies`.

use anyhow::{Result, anyhow};
use serde_json::Value;
use std::sync::OnceLock;

static GLOBAL_TRAVERSER: OnceLock<SchemaTraverser> = OnceLock::new();

pub const SCARB_SCHEMA_JSON: &str = include_str!("../schema.json");

/// Returns a lazily initialised, shared, globally accessible instance of the [`SchemaTraverser`].
pub fn get_shared_traverser() -> &'static SchemaTraverser {
    GLOBAL_TRAVERSER.get_or_init(|| {
        let manifest_schema = get_manifest_schema();
        SchemaTraverser::new(manifest_schema)
    })
}

///  Serializes full JSON Schema for the TomlManifest it into a serde_json::Value.
pub fn get_manifest_schema() -> Value {
    serde_json::from_str(SCARB_SCHEMA_JSON).expect("Failed to serialize Manifest schema")
}

/// Traverses the JSON Schema and returns the definition for a specific TOML path, such as `package.dependencies`.
pub struct SchemaTraverser {
    root: Value,
}

impl SchemaTraverser {
    /// Creates a new SchemaTraverser from the given JSON Schema.
    pub fn new(schema: Value) -> Self {
        Self { root: schema }
    }

    /// Accepts a sequence of keys (e.g. ["package", "dependencies"]) and returns the specific schema node representing that field.
    pub fn traverse<I, S>(&self, path: I) -> Result<&Value>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let path_vec: Vec<String> = path.into_iter().map(|s| s.as_ref().to_owned()).collect();

        let mut current = &self.root;
        let mut iter = path_vec.iter().map(String::as_str).peekable();

        while let Some(key) = iter.next() {
            current = self.resolve_node(current)?;

            let properties = current.get("properties").and_then(|v| v.as_object());

            if let Some(props) = properties {
                current = props.get(key).ok_or_else(|| {
                    anyhow!(
                        "Couldn't resolve '{}' at key '{}'.",
                        path_vec.join("."),
                        key
                    )
                })?;
            } else if iter.peek().is_none() {
                // The last node in the path is valid but does not have properties
                // (e.g. package.edition.workspace)
                return Ok(current);
            } else {
                return Err(anyhow!(
                    "Couldn't resolve '{}' at key '{}'.",
                    path_vec.join("."),
                    key
                ));
            }
        }
        Ok(current)
    }

    /// Handles $ref and anyOf to find the actual object definition
    fn resolve_node<'a>(&'a self, node: &'a Value) -> Result<&'a Value> {
        if let Some(ref_path) = node.get("$ref").and_then(|r| r.as_str()) {
            return self.resolve_ref(ref_path);
        }

        if let Some(any_of) = node.get("anyOf").and_then(|a| a.as_array()) {
            for option in any_of {
                if option.get("properties").is_some() || option.get("$ref").is_some() {
                    return self.resolve_node(option);
                }
            }
        }

        Ok(node)
    }

    /// Basic resolver for "#/$defs/TypeName"
    fn resolve_ref(&self, ref_path: &str) -> Result<&Value> {
        // Instead of repeating a complex object definition multiple times,
        // schemars puts the definition in a central "lookup table" (under $defs) and points to it.
        // Syntax: "$ref": "#/$defs/TypeName"
        let parts: Vec<&str> = ref_path.split('/').collect();
        if parts[0] == "#" && parts[1] == "$defs" {
            return self
                .root
                .get("$defs")
                .and_then(|d| d.get(parts[2]))
                .ok_or_else(|| anyhow!("Definition {} not found", ref_path));
        }
        Err(anyhow!("Unsupported ref format: {}", ref_path))
    }
}
