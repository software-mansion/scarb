use crate::connection::ConnectionManager;
use anyhow::Context;
use starknet_core::codec::{Decode, Encode};
use starknet_core::types::{ByteArray, Felt};
use starknet_core::utils::parse_cairo_short_string;

#[derive(Default, Debug)]
pub struct OracleHintService {
    connections: ConnectionManager,
}

#[derive(Copy, Clone, Debug)]
pub struct OracleCheatcodeSelector(OracleCheatcodeSelectorInner);

#[derive(Copy, Clone, Debug)]
enum OracleCheatcodeSelectorInner {
    OracleInvoke,
}

impl OracleHintService {
    /// Creates a new `OracleHintService`.
    pub fn new() -> Self {
        Self {
            connections: ConnectionManager::new(),
        }
    }

    /// Checks whether this service handles this cheatcode selector.
    pub fn accept_cheatcode(&self, selector: &[u8]) -> Option<OracleCheatcodeSelector> {
        match selector {
            b"oracle_invoke" => Some(OracleCheatcodeSelector(
                OracleCheatcodeSelectorInner::OracleInvoke,
            )),
            _ => None,
        }
    }

    /// Executes the oracle cheatcode.
    ///
    /// Accepts validated cheatcode selector and inputs, returns output.
    /// Any errors at this stage are encoded as in-Cairo `oracle::Result` objects.
    pub fn execute_cheatcode(
        &mut self,
        selector: OracleCheatcodeSelector,
        inputs: &[Felt],
    ) -> Vec<Felt> {
        match selector.0 {
            OracleCheatcodeSelectorInner::OracleInvoke => self.execute_invoke(inputs),
        }
    }

    fn execute_invoke(&mut self, inputs: &[Felt]) -> Vec<Felt> {
        let mut invoke = move || -> anyhow::Result<Vec<Felt>> {
            let mut inputs_iter = inputs.iter();

            let connection_string: String = ByteArray::decode_iter(&mut inputs_iter)?.try_into()?;

            let selector = Felt::decode_iter(&mut inputs_iter)?;
            let selector = parse_cairo_short_string(&selector)
                .with_context(|| format!("invalid selector: {selector}"))?;

            let calldata = inputs_iter.as_slice();

            self.connections
                .connect(&connection_string)?
                .call(&selector, calldata)
        };

        // Encode the result as Result<[felt252; N], ByteArray>. The oracle package interprets this
        // as Result<T, oracle::Error>, where T is user-defined, and oracle::Error has an implicit
        // invariant that it can always deserialise from encoded byte arrays.
        match invoke() {
            Ok(mut result) => {
                result.insert(0, Felt::ZERO);
                result
            }
            Err(err) => {
                let mut result = vec![Felt::ONE];
                let byte_array: ByteArray = format!("{err:?}").as_str().into();
                byte_array
                    .encode(&mut result)
                    .expect("byte array encoding never fails");
                result
            }
        }
    }
}
