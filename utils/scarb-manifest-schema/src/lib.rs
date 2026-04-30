//! Dynamic JSON Schema for Scarb manifest.
//!
//! Provides a way to generate and traverse the Scarb manifest JSON Schema and retrieve the
//! definition for a specific TOML path, such as `package.dependencies`.

use anyhow::{Result, anyhow};
use serde_json::Value;
use std::sync::OnceLock;

static GLOBAL_TRAVERSER: OnceLock<SchemaTraverser> = OnceLock::new();

pub const SCARB_SCHEMA_JSON: &str = include_str!("../schema.json");
pub const SCARB_STRICT_SCHEMA_JSON: &str = include_str!("../schema.strict.json");

/// A TOML field path represented as a sequence of key segments.
pub type FieldPath = Vec<String>;

/// Returns a lazily initialised, shared, globally accessible instance of the [`SchemaTraverser`].
pub fn get_shared_traverser() -> &'static SchemaTraverser {
    GLOBAL_TRAVERSER.get_or_init(|| {
        let manifest_schema = get_manifest_schema();
        SchemaTraverser::new(manifest_schema)
    })
}

/// Deserializes full JSON Schema for the TomlManifest into a serde_json::Value.
pub fn get_manifest_schema() -> Value {
    serde_json::from_str(SCARB_SCHEMA_JSON).expect("Failed to deserialize Manifest schema")
}

/// Deserializes the strict JSON Schema for the TomlManifest into a serde_json::Value.
/// The strict schema has `additionalProperties: false` on every object node, mirroring
/// serde's `deny_unknown_fields`.
pub fn get_strict_manifest_schema() -> Value {
    serde_json::from_str(SCARB_STRICT_SCHEMA_JSON)
        .expect("Failed to deserialize strict Manifest schema")
}

/// Walks a TOML source string and collects all key paths that are not declared in the
/// strict manifest schema (i.e. paths that serde would reject with `deny_unknown_fields`).
///
/// Returns a list of key paths (each path is a [`FieldPath`] of TOML key segments).
/// Validation is performed by the `jsonschema` crate against the strict schema, which has
/// `additionalProperties: false` on every object.  Only `AdditionalProperties` violations
/// are returned; other schema errors (type mismatches, missing required fields, …) are ignored
/// so that this function stays a pure unknown-field detector.
pub fn find_unknown_fields(toml_source: &str) -> Result<Vec<FieldPath>> {
    let toml_value: toml::Value =
        toml::from_str(toml_source).map_err(|e| anyhow!("failed to parse TOML: {e}"))?;

    // Convert toml::Value → serde_json::Value so the JSON Schema validator can work with it.
    let json_value: Value = serde_json::to_value(&toml_value)
        .map_err(|e| anyhow!("failed to convert TOML to JSON: {e}"))?;

    find_unknown_fields_in_json(&get_strict_manifest_schema(), &json_value)
}

fn find_unknown_fields_in_json(schema: &Value, json_value: &Value) -> Result<Vec<FieldPath>> {
    let validator =
        jsonschema::validator_for(schema).map_err(|e| anyhow!("strict schema is invalid: {e}"))?;

    let mut unknown: Vec<FieldPath> = Vec::new();

    for error in validator.iter_errors(json_value) {
        collect_unknown_fields_from_error(&error, &mut unknown);
    }

    Ok(unknown)
}

fn collect_unknown_fields_from_error(
    error: &jsonschema::ValidationError<'_>,
    unknown: &mut Vec<FieldPath>,
) {
    match error.kind() {
        jsonschema::error::ValidationErrorKind::AdditionalProperties { unexpected }
        | jsonschema::error::ValidationErrorKind::UnevaluatedProperties { unexpected } => {
            // `instance_path()` is the JSON Pointer to the object that owns the unexpected keys.
            let base: FieldPath = error
                .instance_path()
                .iter()
                .map(|seg| seg.to_string())
                .collect();

            for field in unexpected {
                let mut path = base.clone();
                path.push(field.clone());
                if !unknown.contains(&path) {
                    unknown.push(path);
                }
            }
        }
        jsonschema::error::ValidationErrorKind::AnyOf { context }
        | jsonschema::error::ValidationErrorKind::OneOfMultipleValid { context }
        | jsonschema::error::ValidationErrorKind::OneOfNotValid { context } => {
            for branch_errors in context {
                for nested in branch_errors {
                    collect_unknown_fields_from_error(nested, unknown);
                }
            }
        }
        jsonschema::error::ValidationErrorKind::PropertyNames { error } => {
            collect_unknown_fields_from_error(error, unknown);
        }
        _ => {}
    }
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
        let path_vec: FieldPath = path.into_iter().map(|s| s.as_ref().to_owned()).collect();

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
