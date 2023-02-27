use std::collections::BTreeMap;
use std::ops::Deref;
use std::sync::Arc;

use smol_str::SmolStr;
use toml::Value;

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

    pub fn is_lib(&self) -> bool {
        matches!(self.kind, TargetKind::Lib(_))
    }
}

impl TargetKind {
    pub fn downcast<K: TargetKindDowncast>(&self) -> &K {
        K::downcast_from(self)
    }

    pub fn name(&self) -> &str {
        match self {
            TargetKind::Lib(_) => "lib",
            TargetKind::External(ExternalTargetKind { kind_name, .. }) => kind_name,
        }
    }
}

#[doc(hidden)]
pub trait TargetKindDowncast {
    fn downcast_from(target_kind: &TargetKind) -> &Self;
}

impl TargetKindDowncast for LibTargetKind {
    fn downcast_from(target_kind: &TargetKind) -> &Self {
        match target_kind {
            TargetKind::Lib(lib) => lib,
            _ => panic!("TargetKind::Lib was expected here"),
        }
    }
}

impl TargetKindDowncast for ExternalTargetKind {
    fn downcast_from(target_kind: &TargetKind) -> &Self {
        match target_kind {
            TargetKind::External(ext) => ext,
            _ => panic!("TargetKind::External was expected here"),
        }
    }
}
