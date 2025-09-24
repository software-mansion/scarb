//! Oracle hint service for Scarb.
//!
//! This crate provides oracle functionality for Cairo programs executed by Scarb.
//! It handles oracle hints and manages connections to external oracle services.
//!
//! ## Tests
//!
//! This crate is e2e tested in `scarb-execute`.

#![deny(clippy::disallowed_methods)]
#![deny(clippy::dbg_macro)]
#![deny(rustdoc::broken_intra_doc_links)]
#![deny(rustdoc::private_intra_doc_links)]

mod assets;
mod connection;
mod connections;
mod hint_service;
mod protocol;

pub use assets::Assets;
pub use connection::Connection;
pub use hint_service::OracleHintService;
pub use protocol::{ConnectCtx, Protocol};
