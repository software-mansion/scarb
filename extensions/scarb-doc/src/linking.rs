use crate::types::item_data::FileLinkDataLocationOffset;
use itertools::Itertools;
use std::path::Path;

/// A data holder necessary for creating documentation links to a remote repository.
#[derive(Clone)]
pub enum RemoteDocLinkingData {
    /// Passed by `scarb doc --remote-base-url=...` command.
    Explicit {
        workspace_root: String,
        remote_base_url: String,
    },
    /// Created based on the manifest-given package repository.
    Manifest {
        repo_root: String,
        commit_hash: String,
        repository_url: String,
    },
    /// Result of a lacking, faulty specification or explicitly disabled.
    Disabled,
}

impl RemoteDocLinkingData {
    pub fn from(
        remote_base_url: Option<String>,
        repo_root: Option<String>,
        workspace_root: Option<String>,
        commit_hash: Option<String>,
        repository_url: Option<String>,
    ) -> Self {
        if remote_base_url.is_some()
            && let Some(workspace_root) = workspace_root
        {
            RemoteDocLinkingData::Explicit {
                workspace_root,
                remote_base_url: remote_base_url.unwrap(),
            }
        } else if let (Some(repo_root), Some(commit_hash), Some(repository_url)) =
            (repo_root, commit_hash, repository_url)
        {
            RemoteDocLinkingData::Manifest {
                repo_root,
                commit_hash: commit_hash.to_owned(),
                repository_url,
            }
        } else {
            RemoteDocLinkingData::Disabled
        }
    }

    pub fn get_formatted_url(
        &self,
        offset: FileLinkDataLocationOffset,
        file_path: &Path,
    ) -> Option<String> {
        let postfix = if let Some((start, end)) = offset {
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
                    .strip_prefix(workspace_root)
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
}
