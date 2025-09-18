use serde::{Deserialize, Serialize};

use crate::{methods::Method, scope::Workspace};

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiscoverWorkspaceParams {
    pub workspace: Workspace,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiscoverWorkspaceResponse {
    pub workspace: Workspace,
}

pub struct DiscoverWorkspace;

impl Method for DiscoverWorkspace {
    const METHOD: &'static str = "discoverWorkspace";

    type Params = DiscoverWorkspaceParams;
    type Response = DiscoverWorkspaceResponse;
}
