//! Basic runner for running a Sierra program on the vm.

use cairo_lang_sierra::extensions::core::{CoreLibfunc, CoreType};
use cairo_lang_sierra::extensions::ConcreteType;
use cairo_lang_sierra::ids::{ConcreteTypeId, GenericTypeId};
use cairo_lang_sierra::program::Function;
use cairo_lang_sierra::program_registry::{ProgramRegistry, ProgramRegistryError};
use cairo_lang_sierra_type_size::{get_type_size_map, TypeSizeMap};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum FinderError {
    #[error("Function with suffix `{suffix}` to run not found.")]
    MissingFunction { suffix: String },
    #[error(transparent)]
    ProgramRegistryError(#[from] Box<ProgramRegistryError>),
    #[error("Unable to create TypeSizeMap.")]
    TypeSizeMapError,
}

pub struct FunctionFinder {
    /// The sierra program.
    sierra_program: cairo_lang_sierra::program::Program,
    /// Program registry for the Sierra program.
    sierra_program_registry: ProgramRegistry<CoreType, CoreLibfunc>,
    // Mapping for the sizes of all types for sierra_program
    type_size_map: TypeSizeMap,
}

#[allow(clippy::result_large_err)]
impl FunctionFinder {
    pub fn new(sierra_program: cairo_lang_sierra::program::Program) -> Result<Self, FinderError> {
        let sierra_program_registry =
            ProgramRegistry::<CoreType, CoreLibfunc>::new(&sierra_program)?;
        let type_size_map = get_type_size_map(&sierra_program, &sierra_program_registry)
            .ok_or(FinderError::TypeSizeMapError)?;

        Ok(Self {
            sierra_program,
            sierra_program_registry,
            type_size_map,
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

    /// Converts array of `ConcreteTypeId`s into corresponding `GenericTypeId`s and their sizes
    pub fn generic_id_and_size_from_concrete(
        &self,
        types: &[ConcreteTypeId],
    ) -> Vec<(GenericTypeId, i16)> {
        types
            .iter()
            .map(|pt| {
                let info = self.get_info(pt);
                let generic_id = &info.long_id.generic_id;
                let size = self.type_size_map[pt];
                (generic_id.clone(), size)
            })
            .collect()
    }
}
