use std::collections::BTreeMap;
use std::ops::Deref;
use std::sync::Arc;

use smol_str::SmolStr;
use toml_edit::easy::Value;

/// See [`TargetInner`] for public fields reference.
#[derive(Clone, Debug)]
pub struct Target(Arc<TargetInner>);

#[derive(Debug)]
#[non_exhaustive]
pub struct TargetInner {
    pub name: SmolStr,
    pub kind: TargetKind,
}

impl Deref for Target {
    type Target = TargetInner;

    fn deref(&self) -> &Self::Target {
        self.0.deref()
    }
}

#[derive(Debug)]
pub enum TargetKind {
    Lib(LibTargetKind),
    External(ExternalTargetKind),
}

#[derive(Debug)]
pub struct LibTargetKind {
    pub sierra: bool,
    pub casm: bool,
}

impl Default for LibTargetKind {
    fn default() -> Self {
        Self {
            sierra: true,
            casm: false,
        }
    }
}

#[derive(Debug)]
pub struct ExternalTargetKind {
    pub kind_name: SmolStr,
    pub params: BTreeMap<SmolStr, Value>,
}

impl Target {
    pub fn new(name: SmolStr, kind: TargetKind) -> Self {
        Self(Arc::new(TargetInner { name, kind }))
    }
}

impl TargetKind {
    pub fn name(&self) -> &str {
        match self {
            TargetKind::Lib(_) => "lib",
            TargetKind::External(ExternalTargetKind { kind_name, .. }) => kind_name,
        }
    }
}
