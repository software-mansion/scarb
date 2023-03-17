use std::ops::Deref;
use std::sync::Arc;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use smol_str::SmolStr;

/// See [`TargetInner`] for public fields reference.
#[derive(Clone, Debug)]
pub struct Target(Arc<TargetInner>);

#[derive(Debug)]
#[non_exhaustive]
pub struct TargetInner {
    pub kind: SmolStr,
    pub name: SmolStr,
    pub params: toml::Value,
}

impl Deref for Target {
    type Target = TargetInner;

    fn deref(&self) -> &Self::Target {
        self.0.deref()
    }
}

impl Target {
    pub const LIB: &'static str = "lib";

    pub fn new(kind: impl Into<SmolStr>, name: impl Into<SmolStr>, params: toml::Value) -> Self {
        assert!(params.is_table(), "params must be a TOML table");
        Self(Arc::new(TargetInner {
            kind: kind.into(),
            name: name.into(),
            params,
        }))
    }

    pub fn without_params(kind: impl Into<SmolStr>, name: impl Into<SmolStr>) -> Self {
        Self::new(kind, name, toml::Value::Table(toml::Table::new()))
    }

    pub fn try_from_structured_params(
        kind: impl Into<SmolStr>,
        name: impl Into<SmolStr>,
        params: impl Serialize,
    ) -> Result<Self> {
        let params = toml::Value::try_from(params)?;
        Ok(Self::new(kind, name, params))
    }

    pub fn is_lib(&self) -> bool {
        self.kind == Self::LIB
    }

    pub fn props<'de, P>(&self) -> Result<P>
    where
        P: Default + Serialize + Deserialize<'de>,
    {
        let mut params = toml::Value::try_from(P::default())?;

        params.as_table_mut().unwrap().extend(
            self.params
                .as_table()
                .unwrap()
                .iter()
                .map(|(k, v)| (k.clone(), v.clone())),
        );

        let props = toml::Value::try_into(params)?;
        Ok(props)
    }
}
