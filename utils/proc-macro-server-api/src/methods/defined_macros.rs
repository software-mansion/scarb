use crate::Method;
use serde::{Deserialize, Serialize};
use std::iter::Sum;

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct DefinedMacrosResponse {
    pub attributes: Vec<String>,
    pub inline_macros: Vec<String>,
    pub derives: Vec<String>,
    pub executables: Vec<String>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct DefinedMacrosParams {}

impl Sum for DefinedMacrosResponse {
    fn sum<T: IntoIterator<Item = Self>>(iter: T) -> Self {
        let mut iter = iter.into_iter();

        let first = iter.next();

        let Some(mut base) = first else {
            return Default::default();
        };

        for other in iter {
            base.attributes.extend(other.attributes);
            base.inline_macros.extend(other.inline_macros);
            base.derives.extend(other.derives);
            base.executables.extend(other.executables);
        }

        base.attributes.sort();
        base.attributes.dedup();
        base.inline_macros.sort();
        base.inline_macros.dedup();
        base.derives.sort();
        base.derives.dedup();
        base.executables.sort();
        base.executables.dedup();

        base
    }
}

pub struct DefinedMacros;

impl Method for DefinedMacros {
    const METHOD: &'static str = "defined-macros";

    type Params = DefinedMacrosParams;
    type Response = DefinedMacrosResponse;
}
