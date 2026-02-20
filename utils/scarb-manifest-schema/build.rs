use std::path::Path;
use std::process;

fn main() {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let schema_path = Path::new(&manifest_dir).join("schema.json");

    if !schema_path.exists() {
        eprintln!(
            "\nERROR: 'schema.json' is missing in {}!\n\
            This file is required to build the crate.\n\
            Please run the generator crate to recreate it:\n\
            \n    cargo run -p scarb-manifest-schema-gen\n",
            manifest_dir
        );
        process::exit(1);
    }
}
