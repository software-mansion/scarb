#[cfg(feature = "execute")]
mod for_execution;

#[cfg(feature = "execute")]
pub use for_execution::*;

// TODO(maciektr): remove when stwo can use the same starknet-types-core version as Cairo.
#[cfg(feature = "execute_unchecked")]
pub mod for_proving;
