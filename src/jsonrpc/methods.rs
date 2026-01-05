//! JSON-RPC method definitions, constants, and registration helpers

use anyhow::Result;
use async_trait::async_trait;

use crate::handlers::{
    generate_fix::{GenerateFixRequest, GenerateFixResponse},
    delete_session::{DeleteSessionRequest, DeleteSessionResponse},
    client_notifications::{ClientNotificationRequest, ClientNotificationResponse},
};
use super::core::{KaiakRequest, KaiakResponse};

/// JSON-RPC method constants for Kaiak procedures
pub const GENERATE_FIX: &str = "kaiak/generate_fix";
pub const GENERATE_FIX_DATA: &str = "kaiak/generate_fix/data";
pub const DELETE_SESSION: &str = "kaiak/delete_session";
pub const CLIENT_USER_MESSAGE: &str = "kaiak/client/user_message";

/// All supported Kaiak JSON-RPC methods
pub const ALL_METHODS: &[&str] = &[GENERATE_FIX, DELETE_SESSION, CLIENT_USER_MESSAGE];

/// Kaiak JSON-RPC trait definition for server implementations
/// Provides type-safe method definitions for all Kaiak procedures
#[async_trait]
pub trait KaiakRpc {
    async fn generate_fix(
        &self,
        request: KaiakRequest<GenerateFixRequest>,
    ) -> Result<KaiakResponse<GenerateFixResponse>, crate::jsonrpc::JsonRpcError>;

    async fn delete_session(
        &self,
        request: KaiakRequest<DeleteSessionRequest>,
    ) -> Result<KaiakResponse<DeleteSessionResponse>, crate::jsonrpc::JsonRpcError>;

    async fn client_user_message(
        &self,
        request: KaiakRequest<ClientNotificationRequest>,
    ) -> Result<KaiakResponse<ClientNotificationResponse>, crate::jsonrpc::JsonRpcError>;
}

