use core::fmt;
use core::result::Result as CoreResult;

pub type Result<T> = CoreResult<T, Error>;

/// An error type that can be raised when invoking oracles.
#[derive(Drop, Clone, PartialEq, Serde)]
pub struct Error {
    message: ByteArray,
}

impl DisplayError of fmt::Display<Error> {
    fn fmt(self: @Error, ref f: fmt::Formatter) -> CoreResult<(), fmt::Error> {
        fmt::Display::fmt(self.message, ref f)
    }
}

impl DebugError of fmt::Debug<Error> {
    fn fmt(self: @Error, ref f: fmt::Formatter) -> CoreResult<(), fmt::Error> {
        write!(f, "oracle::Error({:?})", self.message)
    }
}
