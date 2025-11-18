//! Error types for F1r3fly-RGB operations
//!
//! Minimal, production-ready error handling for F1r3node deployment
//! and RGB contract execution.

use std::error::Error as StdError;
use std::fmt;

/// Core error type for F1r3fly-RGB operations
///
/// Covers network failures, deployment issues, contract execution problems,
/// and state query failures when interacting with F1r3node.
#[derive(Clone, Debug)]
pub enum F1r3flyRgbError {
    /// Failed to connect to F1r3node
    ConnectionFailed(String),

    /// Deployment failed on F1r3node
    DeploymentFailed { deploy_id: String, reason: String },

    /// Invalid response from F1r3node
    InvalidResponse(String),

    /// Contract not found in registry
    ContractNotFound(String),

    /// Invalid method name for contract
    InvalidMethod(String),

    /// Invalid Rholang source code
    InvalidRholangSource(String),

    /// Query execution failed
    QueryFailed(String),

    /// Invalid state format in query response
    InvalidStateFormat(String),

    /// Invalid consignment format
    InvalidConsignment(String),

    /// Serialization error
    SerializationError(String),
}

impl fmt::Display for F1r3flyRgbError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ConnectionFailed(msg) => {
                write!(f, "Failed to connect to F1r3node: {}", msg)
            }
            Self::DeploymentFailed { deploy_id, reason } => {
                write!(
                    f,
                    "Deployment failed: deploy_id={}, reason={}",
                    deploy_id, reason
                )
            }
            Self::InvalidResponse(msg) => {
                write!(f, "Invalid response from F1r3node: {}", msg)
            }
            Self::ContractNotFound(msg) => {
                write!(f, "Contract not found in registry: {}", msg)
            }
            Self::InvalidMethod(method) => {
                write!(f, "Invalid method name: {}", method)
            }
            Self::InvalidRholangSource(msg) => {
                write!(f, "Invalid Rholang source: {}", msg)
            }
            Self::QueryFailed(msg) => {
                write!(f, "Query failed: {}", msg)
            }
            Self::InvalidStateFormat(msg) => {
                write!(f, "Invalid state format: {}", msg)
            }
            Self::InvalidConsignment(msg) => {
                write!(f, "Invalid consignment: {}", msg)
            }
            Self::SerializationError(msg) => {
                write!(f, "Serialization error: {}", msg)
            }
        }
    }
}

impl StdError for F1r3flyRgbError {}

// Helper functions for common error scenarios
impl F1r3flyRgbError {
    /// Create a connection failed error
    pub fn connection_failed(msg: impl Into<String>) -> Self {
        Self::ConnectionFailed(msg.into())
    }

    /// Create a deployment failed error
    pub fn deployment_failed(deploy_id: impl Into<String>, reason: impl Into<String>) -> Self {
        Self::DeploymentFailed {
            deploy_id: deploy_id.into(),
            reason: reason.into(),
        }
    }
}
