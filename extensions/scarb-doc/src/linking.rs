use camino::Utf8PathBuf;
use itertools::Itertools;
use std::ops::Range;
use std::path::{Path, PathBuf};

/// A data holder necessary for creating documentation links to a remote repository.
// #[derive(Clone)]
pub enum RemoteDocLinkingData {
    /// Passed by `scarb doc --remote-base-url=...` command.
    Explicit {
        workspace_root: Utf8PathBuf,
        remote_base_url: String,
    },
    /// Created based on the manifest-given package repository.
    Manifest {
        repo_root: PathBuf,
        commit_hash: String,
        repository_url: String,
    },
    /// Result of a lacking, faulty specification or explicitly disabled.
    Disabled,
}

/// Creates a `RemoteDocLinkingData` based on the given parameters.
pub fn create_remote_doc_linking_data(
    remote_base_url: Option<String>,
    repo_root: Option<PathBuf>,
    workspace_root: Utf8PathBuf,
    commit_hash: Option<String>,
    repository_url: Option<String>,
) -> RemoteDocLinkingData {
    match (remote_base_url, repo_root, commit_hash, repository_url) {
        (Some(remote_base_url), ..) => {
            RemoteDocLinkingData::new_explicit(remote_base_url, workspace_root)
        }
        (None, Some(repo_root), Some(commit_hash), Some(repository_url)) => {
            RemoteDocLinkingData::new_manifest(repo_root, commit_hash, repository_url)
        }
        _ => RemoteDocLinkingData::new_disabled(),
    }
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
                    "<a href='{remote_base_url}{relative_path}{postfix}'> [source code] </a>"
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

    fn new_explicit(remote_base_url: String, workspace_root: Utf8PathBuf) -> Self {
        RemoteDocLinkingData::Explicit {
            workspace_root,
            remote_base_url,
        }
    }

    fn new_manifest(repo_root: PathBuf, commit_hash: String, repository_url: String) -> Self {
        RemoteDocLinkingData::Manifest {
            repo_root,
            commit_hash,
            repository_url,
        }
    }

    fn new_disabled() -> Self {
        RemoteDocLinkingData::Disabled
    }
}
