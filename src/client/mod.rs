//! Client-side functionality for Kaiak
//!
//! This module contains the client-side implementation for communicating
//! with Kaiak servers via JSON-RPC over Unix domain sockets.

pub mod transport;

// Re-export key client types
pub use transport::{JsonRpcClient, ClientInfo};