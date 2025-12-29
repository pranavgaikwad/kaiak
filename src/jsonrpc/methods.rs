//! JSON-RPC method definitions, constants, and registration helpers

use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::handlers::{
    generate_fix::{GenerateFixRequest, GenerateFixResponse},
    delete_session::{DeleteSessionRequest, DeleteSessionResponse},
};
use super::core::{KaiakRequest, KaiakResponse};

/// JSON-RPC method constants for Kaiak procedures
pub const GENERATE_FIX: &str = "kaiak/generate_fix";
pub const DELETE_SESSION: &str = "kaiak/delete_session";

/// All supported Kaiak JSON-RPC methods
pub const ALL_METHODS: &[&str] = &[GENERATE_FIX, DELETE_SESSION];

/// Kaiak JSON-RPC trait definition for server implementations
/// Provides type-safe method definitions for all Kaiak procedures
pub trait KaiakRpc {
    /// Generate a fix for migration issues using AI agent
    ///
    /// Processes migration requests by spawning a Goose agent session with the
    /// provided configuration and workspace settings. Returns agent results
    /// including generated fixes, analysis, and session information.
    async fn generate_fix(
        &self,
        request: KaiakRequest<GenerateFixRequest>,
    ) -> Result<KaiakResponse<GenerateFixResponse>, crate::jsonrpc::JsonRpcError>;

    /// Delete an active agent session
    ///
    /// Terminates an active agent session and cleans up associated resources.
    /// Used to manage session lifecycle and resource cleanup.
    async fn delete_session(
        &self,
        request: KaiakRequest<DeleteSessionRequest>,
    ) -> Result<KaiakResponse<DeleteSessionResponse>, crate::jsonrpc::JsonRpcError>;
}

/// Method registration helper for type-safe method handling
/// Provides utilities for registering methods with consistent validation and error handling
pub struct MethodRegistry;

impl MethodRegistry {
    // TODO: Phase 2.3 - Complete method registration helper implementation
    // For now, we'll use direct method registration in the server implementation
    //
    // /// Register a method with automatic validation and error handling
    // pub fn register_method<P, R, F, Fut>(
    //     server: &mut jsonrpsee::server::ServerBuilder,
    //     method_name: &'static str,
    //     handler: F,
    // ) -> Result<()>
    // where
    //     P: for<'de> Deserialize<'de> + validator::Validate + Send + Sync + 'static,
    //     R: Serialize + Send + Sync + 'static,
    //     F: Fn(KaiakRequest<P>) -> Fut + Send + Sync + 'static,
    //     Fut: std::future::Future<Output = Result<KaiakResponse<R>>> + Send + 'static,
    // {
    //     // Implementation will be completed in Phase 2.3
    //     Ok(())
    // }

    /// Validate that all required methods are registered
    ///
    /// Ensures that all core Kaiak methods are properly registered with the server.
    /// Used during server initialization to verify complete method coverage.
    pub fn validate_complete_registration(registered_methods: &[&str]) -> Result<()> {
        for required_method in ALL_METHODS {
            if !registered_methods.contains(required_method) {
                anyhow::bail!("Required method '{}' is not registered", required_method);
            }
        }
        Ok(())
    }

    /// Get method information for debugging and introspection
    ///
    /// Returns metadata about available methods including their names,
    /// descriptions, and parameter requirements.
    pub fn get_method_info() -> Vec<MethodInfo> {
        vec![
            MethodInfo {
                name: GENERATE_FIX.to_string(),
                description: "Generate fixes for migration issues using Goose AI agent".to_string(),
                params_required: true,
                long_running: true,
            },
            MethodInfo {
                name: DELETE_SESSION.to_string(),
                description: "Delete an active agent session and cleanup resources".to_string(),
                params_required: true,
                long_running: false,
            },
        ]
    }
}

/// Information about a JSON-RPC method for introspection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MethodInfo {
    /// Method name (e.g., "kaiak/configure")
    pub name: String,

    /// Human-readable description of what the method does
    pub description: String,

    /// Whether the method requires parameters
    pub params_required: bool,

    /// Whether the method is expected to run for an extended period
    pub long_running: bool,
}

/// Macro for creating type-safe method handlers
///
/// Simplifies the creation of method handlers with automatic validation
/// and error handling. Used internally by the server implementation.
#[macro_export]
macro_rules! kaiak_method_handler {
    ($handler_fn:expr) => {
        |request| async move {
            $handler_fn(request).await
        }
    };
}
