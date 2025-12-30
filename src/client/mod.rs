//! Client-side functionality for Kaiak
//!
//! This module contains the client-side implementation for communicating
//! with Kaiak servers via JSON-RPC over Unix domain sockets.
//!
//! Uses unified JSON-RPC types from `crate::jsonrpc`.

pub mod transport;

pub use transport::{JsonRpcClient, ClientInfo, ClientRequest, ConnectionState};

pub use crate::jsonrpc::{JsonRpcNotification, JsonRpcRequest, JsonRpcResponse, JsonRpcError};