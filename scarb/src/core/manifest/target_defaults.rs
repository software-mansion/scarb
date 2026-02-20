use crate::core::TargetKind;
use crate::core::{MaybeWorkspace, WorkspaceInherit};
use anyhow::{Result, bail};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::convert::TryFrom;
use std::fmt;

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub struct TargetDefaults {
    pub build_external_contracts: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub struct TomlTargetDefaults {
    pub build_external_contracts: MaybeWorkspaceBuildExternalContracts,
}

impl From<TargetDefaults> for TomlTargetDefaults {
    fn from(value: TargetDefaults) -> Self {
        Self {
            build_external_contracts: MaybeWorkspace::Defined(value.build_external_contracts),
        }
    }
}

pub type MaybeWorkspaceTargetDefaults =
    MaybeWorkspace<TomlTargetDefaults, TomlWorkspaceTargetDefault>;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub struct TomlWorkspaceTargetDefault {
    workspace: bool,
}

impl WorkspaceInherit for TomlWorkspaceTargetDefault {
    fn inherit_toml_table(&self) -> &str {
        "target-defaults"
    }

    fn workspace(&self) -> bool {
        self.workspace
    }
}

pub type MaybeWorkspaceBuildExternalContracts =
    MaybeWorkspace<Vec<String>, TomlWorkspaceBuildExternalContracts>;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub struct TomlWorkspaceBuildExternalContracts {
    workspace: bool,
}

impl WorkspaceInherit for TomlWorkspaceBuildExternalContracts {
    fn inherit_toml_table(&self) -> &str {
        "target-defaults.test"
    }

    fn workspace(&self) -> bool {
        self.workspace
    }
}

#[derive(
    Debug, Clone, Serialize, PartialEq, Eq, Ord, PartialOrd, Hash, Deserialize, JsonSchema,
)]
#[serde(try_from = "TargetKind")]
pub struct TomlTargetKindTestOnly(TargetKind);

impl TomlTargetKindTestOnly {
    #[inline(always)]
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl TryFrom<TargetKind> for TomlTargetKindTestOnly {
    type Error = anyhow::Error;

    fn try_from(value: TargetKind) -> Result<Self> {
        if value.is_test() {
            Ok(Self(value))
        } else {
            bail!(
                "only target kind `test` is allowed in `target_defaults`, but found `{}`",
                value
            );
        }
    }
}

impl From<TomlTargetKindTestOnly> for TargetKind {
    fn from(value: TomlTargetKindTestOnly) -> Self {
        value.0
    }
}

impl fmt::Display for TomlTargetKindTestOnly {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}
