use scarb::core::TomlManifest;
use serde_json::Value;
use std::fs;
use std::path::PathBuf;

fn main() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    eprintln!("Generating schema for manifest in {:?}", manifest_dir);

    let schema_dir = manifest_dir
        .parent()
        .unwrap_or_else(|| {
            panic!(
                "Failed to find parent directory of manifest: {:?}",
                manifest_dir
            )
        })
        .join("scarb-manifest-schema");

    let schema = schemars::schema_for!(TomlManifest);

    let json = serde_json::to_string_pretty(&schema).expect("Failed to serialize schema");
    fs::write(schema_dir.join("schema.json"), &json).expect("Failed to write schema");
    eprintln!("Schema updated at: {:?}", schema_dir.join("schema.json"));

    let schema_value: Value =
        serde_json::to_value(&schema).expect("Failed to convert schema to Value");

    let strict = make_strict(schema_value);
    let strict_json =
        serde_json::to_string_pretty(&strict).expect("Failed to serialize strict schema");
    fs::write(schema_dir.join("schema.strict.json"), strict_json)
        .expect("Failed to write strict schema");
    eprintln!(
        "Strict schema updated at: {:?}",
        schema_dir.join("schema.strict.json")
    );
}

/// Transforms a JSON Schema value into a "strict" variant that disallows additional properties
/// on every object schema that has a `properties` field but no existing `additionalProperties`.
/// This mirrors serde's `deny_unknown_fields` behaviour.
pub fn make_strict(mut schema: Value) -> Value {
    make_strict_recursive(&mut schema);
    schema
}

fn make_strict_recursive(value: &mut Value) {
    match value {
        Value::Object(obj) => {
            if obj.contains_key("properties") && !obj.contains_key("additionalProperties") {
                obj.insert("additionalProperties".to_string(), Value::Bool(false));
            }
            for v in obj.values_mut() {
                make_strict_recursive(v);
            }
        }
        Value::Array(arr) => {
            for v in arr.iter_mut() {
                make_strict_recursive(v);
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use indoc::formatdoc;
    use scarb::core::TomlManifest;
    use scarb_manifest_schema::{SCARB_SCHEMA_JSON, SCARB_STRICT_SCHEMA_JSON};
    use schemars::schema_for;
    use serde_json::Value;

    use super::make_strict;

    #[test]
    fn test_schema_is_up_to_date() {
        let current_schema = schema_for!(TomlManifest);
        let current_json = serde_json::to_string_pretty(&current_schema)
            .expect("Failed to serialize current schema");

        // Normalize line endings to LF before comparing, as git may check out
        // schema.json with CRLF on Windows (core.autocrlf=true).
        let current_json = current_json.replace("\r\n", "\n");
        let schema_json = SCARB_SCHEMA_JSON.replace("\r\n", "\n");

        if current_json != schema_json {
            panic!(
                "{}",
                formatdoc! {"
            ERROR: Scarb manifest schema is out of date!

            TO FIX THIS, run the following command:

                cargo run -p scarb-manifest-schema-gen

            This will refresh the snapshot and fix this test.
        "}
            );
        }
    }

    #[test]
    fn test_strict_schema_is_up_to_date() {
        let current_schema = schema_for!(TomlManifest);
        let current_value: Value =
            serde_json::to_value(&current_schema).expect("Failed to convert schema to Value");
        let current_strict = make_strict(current_value);
        let current_json = serde_json::to_string_pretty(&current_strict)
            .expect("Failed to serialize current strict schema");

        let current_json = current_json.replace("\r\n", "\n");
        let strict_json = SCARB_STRICT_SCHEMA_JSON.replace("\r\n", "\n");

        if current_json != strict_json {
            panic!(
                "{}",
                formatdoc! {"
            ERROR: Scarb manifest strict schema is out of date!

            TO FIX THIS, run the following command:

                cargo run -p scarb-manifest-schema-gen

            This will refresh the snapshot and fix this test.
        "}
            );
        }
    }
}
