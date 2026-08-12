use crate::core::PackageId;
use crate::core::Workspace;
use crate::flock::LockedFile;
use crate::ops::subcommands::execute_external_subcommand_and_wait;
use anyhow::{Context, Result};
use camino::Utf8Path;
use camino::Utf8PathBuf;
use scarb_ui::HumanBytes;
use scarb_ui::components::Status;
use std::fs::File;
use std::io::Seek;
use std::io::SeekFrom;

fn generate_docs(pkg_id: &PackageId, ws: &Workspace<'_>) -> Result<Utf8PathBuf> {
    let docs_target = ws.target_dir().path_unchecked().to_owned();
    let config = ws.config();
    let args = vec![
        "--build".into(),
        "--disable-remote-linking".into(),
        "--package".into(),
        pkg_id.name.to_string().into(),
    ];

    execute_external_subcommand_and_wait("doc", &args, None, config, Some(docs_target.clone()))?;

    let docs_path = docs_target
        .join("doc")
        .join(pkg_id.name.to_string())
        .join("book");
    Ok(docs_path)
}

fn tar(src: &Utf8Path, dst: &mut File) -> Result<()> {
    const COMPRESSION_LEVEL: i32 = 22;
    let encoder = zstd::stream::Encoder::new(dst, COMPRESSION_LEVEL)?;

    let mut tar = tar::Builder::new(encoder);
    tar.append_dir_all(".", src)
        .with_context(|| format!("failed to append directory all for {}", src))?;

    let encoder = tar.into_inner()?;

    encoder.finish()?;
    Ok(())
}

pub fn package_docs_one(package_id: &PackageId, ws: &Workspace<'_>) -> Result<LockedFile> {
    let docs_path = generate_docs(package_id, ws)?;

    let filename = format!("docs.{}", package_id.tarball_name());
    let target_dir = ws.target_dir().child("doc");

    let mut dst = target_dir.create_rw(&filename, "docs tarball", ws.config())?;

    tar(&docs_path, &mut dst)?;
    let uncompressed_size = 0; //TODO(hakiers)

    dst.seek(SeekFrom::Start(0))?;
    let dst_metadata = dst
        .metadata()
        .with_context(|| format!("failed to get metadata for {}", dst.path()))?;
    let compressed_size = dst_metadata.len();

    ws.config().ui().print(Status::new(
        "Packaged",
        &format!(
            "docs for {} (uncompressed size: {}, compressed size: {})",
            package_id.name,
            HumanBytes(uncompressed_size),
            HumanBytes(compressed_size),
        ),
    ));

    Ok(dst)
}
