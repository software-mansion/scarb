use core::fmt::Debug;
use core::serde::Serde;

/// Poor man's implementation of oracle invocation result deserialization.
pub fn deserialize<T, +Serde<T>, +Destruct<Result<T, ByteArray>>>(
    mut value: Span<felt252>,
) -> Result<T, ByteArray> {
    Serde::<Result<T, ByteArray>>::deserialize(ref value).unwrap_or(Err("serde failed"))
}

/// Prints a `Span<felt252>` by first trying to deserialize it as `Result<T, ByteArray>`.
/// If deserialization fails, prints the raw span values.
/// Returns the original span unchanged.
pub fn print<T, +Serde<T>, +Debug<T>, +Drop<T>>(value: Span<felt252>) -> Span<felt252> {
    // Create a mutable copy of the value for deserialization attempt.
    let mut value_copy = value;

    match Serde::<Result<T, ByteArray>>::deserialize(ref value_copy) {
        Option::Some(result) => { println!("{:?}", result); },
        Option::None => { println!("{:?}", value); },
    }

    value
}
