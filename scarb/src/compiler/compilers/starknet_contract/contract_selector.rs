use crate::core::PackageName;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

pub const CAIRO_PATH_SEPARATOR: &str = "::";
pub const GLOB_PATH_SELECTOR: &str = "*";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContractSelector(pub String);

impl ContractSelector {
    pub fn package(&self) -> PackageName {
        let parts = self
            .0
            .split_once(CAIRO_PATH_SEPARATOR)
            .unwrap_or((self.0.as_str(), ""));
        PackageName::new(parts.0)
    }

    pub fn contract(&self) -> String {
        let parts = self
            .0
            .rsplit_once(CAIRO_PATH_SEPARATOR)
            .unwrap_or((self.0.as_str(), ""));
        parts.1.to_string()
    }

    pub fn is_wildcard(&self) -> bool {
        self.0.ends_with(GLOB_PATH_SELECTOR)
    }

    pub fn partial_path(&self) -> String {
        let parts = self
            .0
            .split_once(GLOB_PATH_SELECTOR)
            .unwrap_or((self.0.as_str(), ""));
        parts.0.to_string()
    }

    pub fn full_path(&self) -> String {
        self.0.clone()
    }
}

pub struct ContractFileStemCalculator(HashSet<String>);

impl ContractFileStemCalculator {
    pub fn new(contract_paths: Vec<String>) -> Self {
        let mut seen = HashSet::new();
        let contract_name_duplicates = contract_paths
            .iter()
            .map(|it| ContractSelector(it.clone()).contract())
            .filter(|contract_name| {
                // insert returns false for duplicate values
                !seen.insert(contract_name.clone())
            })
            .collect::<HashSet<String>>();
        Self(contract_name_duplicates)
    }

    pub fn get_stem(&mut self, full_path: String) -> String {
        let contract_selector = ContractSelector(full_path);
        let contract_name = contract_selector.contract();

        if self.0.contains(&contract_name) {
            contract_selector
                .full_path()
                .replace(CAIRO_PATH_SEPARATOR, "_")
        } else {
            contract_name
        }
    }
}
