use crate::protocol::Protocols;

#[cfg(feature = "shell")]
mod shell;
#[cfg(feature = "wasm")]
mod wasm;

pub fn builtin_protocols() -> Protocols {
    let mut p = Protocols::default();

    #[cfg(feature = "shell")]
    p.add::<shell::Shell>();
    #[cfg(feature = "wasm")]
    p.add::<wasm::Wasm>();

    p
}
