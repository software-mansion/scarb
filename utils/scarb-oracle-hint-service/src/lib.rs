//! Oracle hint service for Scarb.
//!
//! This crate provides oracle functionality for Cairo programs executed by Scarb.
//! It handles oracle hints and manages connections to external oracle services.
//!
//! ## Tests
//!
//! This crate is e2e tested in `scarb-execute`.

mod connection;
mod connections;
mod hint_service;
mod jsonrpc;

pub use hint_service::OracleHintService;
