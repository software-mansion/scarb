//! Dynamic JSON Schema for Scarb manifest.
//!
//! Provides a way to generate and traverse the Scarb manifest JSON Schema and retrieve the
//! definition for a specific TOML path, such as `package.dependencies`.

use anyhow::{Result, anyhow};
use scarb::core::TomlManifest;
use schemars::schema_for;
use serde_json::Value;

/// Generates the full JSON Schema for the TomlManifest and serializes it into a serde_json::Value.
pub fn get_manifest_schema() -> Value {
    serde_json::to_value(schema_for!(TomlManifest)).expect("Failed to serialize Manifest schema")
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
    pub fn traverse(&self, path: Vec<&str>) -> Result<&Value> {
        let mut current = &self.root;

        for key in path.iter() {
            current = self.resolve_node(current)?;

            let properties = current
                .get("properties")
                .and_then(|v| v.as_object())
                .ok_or_else(|| anyhow!("Node at '{}' does not have properties", key))?;

            current = properties
                .get(*key)
                .ok_or_else(|| anyhow!("Property '{}' not found in schema", key))?;
        }
        // Prefer the field node over the object node.
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
