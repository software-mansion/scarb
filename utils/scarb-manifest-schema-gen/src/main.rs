use scarb::core::TomlManifest;
use std::fs;
use std::path::PathBuf;

fn main() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    eprintln!("Generating schema for manifest in {:?}", manifest_dir);

    let output_path = manifest_dir
        .parent()
        .unwrap_or_else(|| {
            panic!(
                "Failed to find parent directory of manifest: {:?}",
                manifest_dir
            )
        })
        .join("scarb-manifest-schema")
        .join("schema.json");

    let schema = schemars::schema_for!(TomlManifest);
    let json = serde_json::to_string_pretty(&schema).expect("Failed to serialize schema");

    fs::write(&output_path, json).expect("Failed to write schema");
    eprintln!("Schema updated at: {:?}", output_path);
}

#[cfg(test)]
mod tests {
    use indoc::formatdoc;
    use scarb::core::TomlManifest;
    use scarb_manifest_schema::SCARB_SCHEMA_JSON;
    use schemars::schema_for;

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
}
