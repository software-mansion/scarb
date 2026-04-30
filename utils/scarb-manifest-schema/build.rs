use std::path::Path;
use std::process;

fn main() {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();

    for name in ["schema.json", "schema.strict.json"] {
        let schema_path = Path::new(&manifest_dir).join(name);
        if !schema_path.exists() {
            eprintln!(
                "\nERROR: '{name}' is missing in {manifest_dir}!\n\
                This file is required to build the crate.\n\
                Please run the generator crate to recreate it:\n\
                \n    cargo run -p scarb-manifest-schema-gen\n",
            );
            process::exit(1);
        }
    }
}
