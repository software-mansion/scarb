use camino::Utf8PathBuf;
use scarb_metadata::Metadata;

pub mod compilation;

pub fn get_target_dir(metadata: &Metadata) -> Utf8PathBuf {
    metadata
        .target_dir
        .clone()
        .unwrap_or_else(|| metadata.workspace.root.join("target"))
}
