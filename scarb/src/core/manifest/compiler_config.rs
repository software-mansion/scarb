use serde::{Deserialize, Serialize};
use std::hash::Hash;

use crate::compiler::{DefaultForProfile, Profile};
use crate::core::TomlCairo;

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq, Hash)]
pub struct ManifestCompilerConfig {
    /// Replace all names in generated Sierra code with dummy counterparts, representing the
    /// expanded information about the named items.
    ///
    /// For libfuncs and types that would be recursively opening their generic arguments.
    /// For functions, that would be their original name in Cairo.
    /// For example, while the Sierra name be `[6]`, with this flag turned on it might be:
    /// - For libfuncs: `felt252_const<2>` or `unbox<Box<Box<felt252>>>`.
    /// - For types: `felt252` or `Box<Box<felt252>>`.
    /// - For user functions: `test::foo`.
    pub sierra_replace_ids: bool,
    /// Do not exit with error on compiler warnings.
    pub allow_warnings: bool,
    /// Enable auto gas withdrawal and gas usage check.
    pub enable_gas: bool,
    /// Add a mapping between sierra statement indexes and fully qualified paths of cairo functions
    /// to debug info. A statement index maps to a vector consisting of a function which caused the
    /// statement to be generated and all functions that were inlined or generated along the way.
    /// Used by [cairo-profiler](https://github.com/software-mansion/cairo-profiler).
    /// This feature is unstable and is subject to change.
    pub unstable_add_statements_functions_debug_info: bool,
    /// Add a mapping between sierra statement indexes and code location in cairo code
    /// to debug info. A statement index maps to a vector consisting of a code location which caused the
    /// statement to be generated and all code location that were inlined or generated along the way.
    /// Used by [cairo-coverage](https://github.com/software-mansion/cairo-coverage).
    /// This feature is unstable and is subject to change.
    pub unstable_add_statements_code_locations_debug_info: bool,
    /// Whether to add panic backtrace handling to the generated code.
    pub panic_backtrace: bool,
    /// Do not generate panic handling code. This might be useful for client side proving.  
    pub unsafe_panic: bool,
    /// Inlining strategy.
    pub inlining_strategy: InliningStrategy,
    /// Whether to enable incremental compilation.
    ///
    /// If this is set to `true`, the compiler will emit compilation artifacts and attempt to reuse
    /// them in subsequent builds.
    pub incremental: bool,
}

#[derive(Debug, Default, Deserialize, Serialize, Eq, PartialEq, Hash, Clone)]
#[serde(
    rename_all = "kebab-case",
    try_from = "serdex::InliningStrategy",
    into = "serdex::InliningStrategy"
)]
pub enum InliningStrategy {
    /// Do not override inlining strategy.
    #[default]
    Default,
    /// Inline only in the case of an `inline(always)` annotation.
    Avoid,
    /// Should inline small functions up to the given weight.
    ///
    /// Note: the weight exact definition is subject to change.
    InlineSmallFunctions(usize),
}

mod serdex {
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Deserialize, Serialize)]
    #[serde(untagged)]
    pub enum InliningStrategy {
        InlineSmallFunctions(usize),
        Predefined(String),
    }

    impl TryFrom<InliningStrategy> for super::InliningStrategy {
        type Error = serde::de::value::Error;

        fn try_from(value: InliningStrategy) -> Result<Self, Self::Error> {
            match value {
                InliningStrategy::InlineSmallFunctions(weight) => {
                    Ok(Self::InlineSmallFunctions(weight))
                }
                InliningStrategy::Predefined(name) => match name.as_str() {
                    "default" => Ok(Self::Default),
                    "release" => Ok(Self::Default),
                    "avoid" => Ok(Self::Avoid),
                    _ => Err(serde::de::Error::custom(format!(
                        "unknown inlining strategy: `{name}`\nuse one of: `default`, `avoid` or a number"
                    ))),
                },
            }
        }
    }

    impl From<super::InliningStrategy> for InliningStrategy {
        fn from(strategy: super::InliningStrategy) -> Self {
            match strategy {
                super::InliningStrategy::Default => Self::Predefined("default".to_string()),
                super::InliningStrategy::Avoid => Self::Predefined("avoid".to_string()),
                super::InliningStrategy::InlineSmallFunctions(weight) => {
                    Self::InlineSmallFunctions(weight)
                }
            }
        }
    }
}

impl DefaultForProfile for ManifestCompilerConfig {
    fn default_for_profile(profile: &Profile) -> Self {
        Self {
            sierra_replace_ids: profile.is_dev(),
            allow_warnings: true,
            enable_gas: true,
            unstable_add_statements_functions_debug_info: false,
            unstable_add_statements_code_locations_debug_info: false,
            panic_backtrace: false,
            unsafe_panic: false,
            inlining_strategy: InliningStrategy::default(),
            incremental: true,
        }
    }
}

impl From<ManifestCompilerConfig> for TomlCairo {
    fn from(config: ManifestCompilerConfig) -> Self {
        Self {
            sierra_replace_ids: Some(config.sierra_replace_ids),
            allow_warnings: Some(config.allow_warnings),
            enable_gas: Some(config.enable_gas),
            unstable_add_statements_functions_debug_info: Some(
                config.unstable_add_statements_functions_debug_info,
            ),
            unstable_add_statements_code_locations_debug_info: Some(
                config.unstable_add_statements_code_locations_debug_info,
            ),
            panic_backtrace: Some(config.panic_backtrace),
            unsafe_panic: Some(config.unsafe_panic),
            inlining_strategy: Some(config.inlining_strategy),
            incremental: Some(config.incremental),
        }
    }
}
