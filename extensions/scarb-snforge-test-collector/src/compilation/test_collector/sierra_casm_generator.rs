//! Basic runner for running a Sierra program on the vm.

use cairo_lang_sierra::extensions::core::{CoreLibfunc, CoreType};
use cairo_lang_sierra::extensions::ConcreteType;
use cairo_lang_sierra::program::Function;
use cairo_lang_sierra::program_registry::{ProgramRegistry, ProgramRegistryError};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum FinderError {
    #[error("Function with suffix `{suffix}` to run not found.")]
    MissingFunction { suffix: String },
    #[error(transparent)]
    ProgramRegistryError(#[from] Box<ProgramRegistryError>),
}

pub struct FunctionFinder {
    /// The sierra program.
    sierra_program: cairo_lang_sierra::program::Program,
    /// Program registry for the Sierra program.
    sierra_program_registry: ProgramRegistry<CoreType, CoreLibfunc>,
}

#[allow(clippy::result_large_err)]
impl FunctionFinder {
    pub fn new(sierra_program: cairo_lang_sierra::program::Program) -> Result<Self, FinderError> {
        let sierra_program_registry =
            ProgramRegistry::<CoreType, CoreLibfunc>::new(&sierra_program)?;
        Ok(Self {
            sierra_program,
            sierra_program_registry,
        })
    }

    // Copied from crates/cairo-lang-runner/src/lib.rs
    /// Finds first function ending with `name_suffix`.
    pub fn find_function(&self, name_suffix: &str) -> Result<&Function, FinderError> {
        self.sierra_program
            .funcs
            .iter()
            .find(|f| {
                if let Some(name) = &f.id.debug_name {
                    name.ends_with(name_suffix)
                } else {
                    false
                }
            })
            .ok_or_else(|| FinderError::MissingFunction {
                suffix: name_suffix.to_owned(),
            })
    }

    #[must_use]
    pub fn get_info(
        &self,
        ty: &cairo_lang_sierra::ids::ConcreteTypeId,
    ) -> &cairo_lang_sierra::extensions::types::TypeInfo {
        self.sierra_program_registry.get_type(ty).unwrap().info()
    }
}
