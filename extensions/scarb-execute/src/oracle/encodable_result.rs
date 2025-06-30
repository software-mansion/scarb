use starknet_core::codec::{Encode, Error as CodecError, FeltWriter};
use starknet_core::types::{ByteArray, Felt};

pub enum EncodableResult<T, E> {
    Ok(T),
    Err(E),
}

impl<T, E> From<Result<T, E>> for EncodableResult<T, E> {
    fn from(result: Result<T, E>) -> Self {
        match result {
            Ok(value) => EncodableResult::Ok(value),
            Err(error) => EncodableResult::Err(error),
        }
    }
}

impl<T, E> Encode for EncodableResult<T, E>
where
    T: Encode,
    E: ToString,
{
    fn encode<W: FeltWriter>(&self, writer: &mut W) -> Result<(), CodecError> {
        match self {
            EncodableResult::Ok(value) => {
                writer.write(Felt::ZERO);
                value.encode(writer)
            }
            EncodableResult::Err(value) => {
                writer.write(Felt::ONE);
                let byte_array: ByteArray = value.to_string().as_str().into();
                byte_array.encode(writer)
            }
        }
    }
}
