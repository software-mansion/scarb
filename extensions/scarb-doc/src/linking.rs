use anyhow::{bail, ensure};
use camino::Utf8PathBuf;
use indoc::formatdoc;
use itertools::Itertools;
use scarb_ui::Ui;
use std::ops::Range;
use std::path::{Path, PathBuf};

/// A data holder necessary for creating documentation links to a remote repository.
#[derive(Clone)]
pub enum RemoteDocLinkingData {
    /// Passed by `scarb doc --remote-base-url=...` command.
    Explicit {
        workspace_root: Utf8PathBuf,
        remote_base_url: String,
    },
    /// Created based on manifest given package repository.
    Manifest {
        repo_root: PathBuf,
        commit_hash: String,
        repository_url: String,
    },
    /// Result of a lacking, faulty specification or (not yet implemented) explicitly disabled.
    Disabled,
}

impl RemoteDocLinkingData {
    pub fn get_formatted_url(
        &self,
        offset: Option<Range<usize>>,
        file_path: &Path,
    ) -> Option<String> {
        let postfix = if let Some(Range { start, end }) = offset {
            format!("#L{}-L{}", start + 1, end + 1) // +1 because of the indexing difference between compiler and GitHub url resolving
        } else {
            "".to_string()
        };
        match self {
            RemoteDocLinkingData::Explicit {
                remote_base_url,
                workspace_root,
            } => {
                let relative_path = file_path
                    .strip_prefix(workspace_root.as_std_path())
                    .ok()?
                    .components()
                    .filter_map(|c| c.as_os_str().to_str())
                    .join("/");
                Some(format!(
                    "<a href='{}/{relative_path}{postfix}'> [source code] </a>",
                    remote_base_url.trim_end_matches('/')
                ))
            }
            RemoteDocLinkingData::Manifest {
                repo_root,
                commit_hash,
                repository_url,
            } => {
                let relative_path = file_path
                    .strip_prefix(repo_root)
                    .ok()?
                    .components()
                    .filter_map(|c| c.as_os_str().to_str())
                    .join("/");
                Some(format!(
                    "<a href='{repository_url}/blob/{commit_hash}/{relative_path}{postfix}'> [source code] </a>"
                ))
            }
            RemoteDocLinkingData::Disabled => None,
        }
    }

    pub fn new_explicit(remote_base_url: String, workspace_root: Utf8PathBuf) -> Self {
        RemoteDocLinkingData::Explicit {
            workspace_root,
            remote_base_url,
        }
    }

    pub fn new_manifest(repo_root: PathBuf, commit_hash: String, repository_url: String) -> Self {
        RemoteDocLinkingData::Manifest {
            repo_root,
            commit_hash,
            repository_url,
        }
    }

    pub fn new_disabled() -> Self {
        RemoteDocLinkingData::Disabled
    }
}

pub fn discover_repo_ctx(workspace_root: &Utf8PathBuf) -> (Option<PathBuf>, Option<String>) {
    match gix::discover(workspace_root) {
        Ok(repo) => (
            repo.workdir().map(Path::to_path_buf),
            repo.rev_parse_single("HEAD").ok().map(|h| h.to_string()),
        ),
        Err(_) => (None, None),
    }
}

pub fn resolve_remote_linking_data(
    ui: &Ui,
    workspace_root: &Utf8PathBuf,
    repo_root: &Option<PathBuf>,
    commit_hash: &Option<String>,
    disable_linking: bool,
    remote_base_url: &Option<String>,
    manifest_repo_url: &Option<String>,
) -> anyhow::Result<RemoteDocLinkingData> {
    if disable_linking {
        return Ok(RemoteDocLinkingData::new_disabled());
    }
    ensure!(
        remote_base_url.is_some() || manifest_repo_url.is_some(),
        formatdoc! {r#"
            remote source linking is enabled, but no repository URL is configured,
            provide `--remote-base-url` or pass `--disable-remote-linking`,
            see https://docs.swmansion.com/scarb/docs/extensions/documentation-generation.html#linking-to-the-source-code-vcs-repository for details
        "#}
    );

    if manifest_repo_url.is_some() && remote_base_url.is_some() {
        ui.warn("both `--remote-base-url` and manifest repository URL provided, using the `--remote-base-url` URL for remote linking");
    }

    match (&repo_root, &commit_hash, manifest_repo_url, remote_base_url) {
        (_, _, _, Some(base_url)) => Ok(RemoteDocLinkingData::new_explicit(
            base_url.clone(),
            workspace_root.clone(),
        )),
        (Some(repo_root), Some(commit_hash), Some(repo_url), _) => {
            Ok(RemoteDocLinkingData::new_manifest(
                repo_root.clone(),
                commit_hash.clone(),
                repo_url.clone(),
            ))
        }
        _ => bail!("could not discover a Git repository, remote linking disabled"),
    }
}
