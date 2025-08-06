use crate::MANIFEST_FILE_NAME;
use crate::compiler::incremental::fingerprint::LocalFingerprint;
use crate::internal::fsx;
use crate::internal::fsx::PathUtf8Ext;
use cairo_lang_filesystem::ids::CAIRO_FILE_EXTENSION;
use camino::Utf8PathBuf;
use ignore::WalkState::Continue;
use ignore::types::TypesBuilder;
use ignore::{DirEntry, WalkBuilder};
use indoc::formatdoc;
use itertools::Itertools;
use scarb_stable_hash::u64_hash;
use scarb_ui::Ui;
use smol_str::SmolStr;
use std::sync::atomic::{AtomicBool, Ordering};
use tracing::warn;

#[tracing::instrument(skip_all, level = "info")]
pub(crate) fn create_local_fingerprints(
    source_paths: Vec<Utf8PathBuf>,
    target_name: SmolStr,
    ui: Ui,
) -> Vec<LocalFingerprint> {
    let source_paths = source_paths
        .into_iter()
        .map(|p| {
            if !p.is_dir() {
                p.parent()
                    .expect("source path must have a parent")
                    .to_path_buf()
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
        .skip_stdout(true)
        .types(types_builder.build().unwrap())
        // Skip any subdirectories containing `Scarb.toml`.
        .filter_entry(|entry: &DirEntry| !entry.path().join(MANIFEST_FILE_NAME).exists())
        .build_parallel();

    let (tx, rx) = std::sync::mpsc::channel();
    let any_missed = AtomicBool::new(false);

    walker.run(|| {
        let tx = tx.clone();
        let any_missed = &any_missed;
        Box::new(move |result| {
            let dir_entry = if let Ok(dir_entry) = result {
                dir_entry
            } else {
                warn!("failed to read the file");
                any_missed.store(true, Ordering::Release);
                return Continue;
            };

            let file_type = if let Some(file_type) = dir_entry.file_type() {
                file_type
            } else {
                warn!("failed to read filetype");
                any_missed.store(true, Ordering::Release);
                return Continue;
            };

            let path = dir_entry.path();

            if !file_type.is_file() {
                return Continue;
            }

            let Ok(path) = path.try_to_utf8() else {
                warn!("failed to convert path to UTF-8: {}", path.display());
                any_missed.store(true, Ordering::Release);
                return Continue;
            };

            let Ok(content) = fsx::read_to_string(&path) else {
                warn!("failed to read file: {}", path.to_string());
                any_missed.store(true, Ordering::Release);
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

    if any_missed.load(Ordering::Acquire) {
        ui.warn(formatdoc! {r#"
            some files were skipped when calculating source checksums for `{target_name}` component
            this may or may not affect the build and incremental compilation cache loading
            please run Scarb with `--verbose` to see which files were skipped
        "#});
    }

    fingerprints
}
