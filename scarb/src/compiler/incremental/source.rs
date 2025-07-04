use crate::compiler::incremental::fingerprint::LocalFingerprint;
use crate::internal::fsx;
use crate::internal::fsx::PathUtf8Ext;
use crate::{DEFAULT_TARGET_DIR_NAME, MANIFEST_FILE_NAME};
use cairo_lang_filesystem::ids::CAIRO_FILE_EXTENSION;
use camino::Utf8Path;
use ignore::WalkState::Continue;
use ignore::types::TypesBuilder;
use ignore::{DirEntry, WalkBuilder};
use itertools::Itertools;
use scarb_stable_hash::u64_hash;
use tracing::warn;

#[tracing::instrument(skip_all, level = "info")]
pub(crate) fn create_local_fingerprints(source_paths: Vec<&Utf8Path>) -> Vec<LocalFingerprint> {
    let filter = {
        move |entry: &DirEntry| -> bool {
            let path = entry.path();
            let is_root = entry.depth() == 0;

            // Skip any subdirectories containing `Scarb.toml`.
            if !is_root && path.join(MANIFEST_FILE_NAME).exists() {
                return false;
            }

            // Skip `target` directory.
            if entry.depth() == 1
                && ({
                    let f = entry.file_name();
                    f == DEFAULT_TARGET_DIR_NAME
                })
            {
                return false;
            }

            true
        }
    };
    let source_paths = source_paths
        .into_iter()
        .map(|p| {
            if !p.is_dir() {
                p.parent().expect("source path must have a parent")
            } else {
                p
            }
        })
        .dedup()
        .collect_vec();
    let Some((first, rest)) = source_paths.split_first() else {
        return Vec::new();
    };
    let mut builder = WalkBuilder::new(first);
    for path in rest {
        builder.add(path);
    }
    let mut types_builder = TypesBuilder::new();
    types_builder
        .add(CAIRO_FILE_EXTENSION, &format!("*.{CAIRO_FILE_EXTENSION}"))
        .unwrap();
    types_builder.select(CAIRO_FILE_EXTENSION);
    let walker = builder
        .follow_links(true)
        .standard_filters(false)
        .parents(false)
        .same_file_system(false)
        .filter_entry(filter)
        .skip_stdout(true)
        .types(types_builder.build().unwrap())
        .build_parallel();

    let (tx, rx) = std::sync::mpsc::channel();

    walker.run(|| {
        let tx = tx.clone();
        Box::new(move |result| {
            let dir_entry = if let Ok(dir_entry) = result {
                dir_entry
            } else {
                warn!("failed to read the file");
                return Continue;
            };

            let file_type = if let Some(file_type) = dir_entry.file_type() {
                file_type
            } else {
                warn!("failed to read filetype");
                return Continue;
            };

            let path = dir_entry.path();

            if !file_type.is_file() {
                return Continue;
            }

            let Ok(path) = path.try_to_utf8() else {
                warn!("failed to convert path to UTF-8: {}", path.display());
                return Continue;
            };

            let Ok(content) = fsx::read_to_string(&path) else {
                warn!("failed to read file: {}", path.to_string());
                return Continue;
            };

            let checksum = u64_hash(content.as_bytes());

            tx.send((path, checksum))
                .expect("channel closed unexpectedly");

            Continue
        })
    });

    drop(tx);

    let mut fingerprints = Vec::new();
    while let Ok((path, checksum)) = rx.recv() {
        fingerprints.push(LocalFingerprint { path, checksum });
    }
    fingerprints
}
