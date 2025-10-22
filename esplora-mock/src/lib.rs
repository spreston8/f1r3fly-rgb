/// Esplora Mock Server Library
/// 
/// This crate provides both a standalone binary and library components
/// for mocking Esplora API with Bitcoin Core RPC backend.

pub mod handlers;
pub mod rpc_client;
pub mod server;
pub mod types;

// Re-export commonly used types
pub use rpc_client::BitcoinRpcClient;
pub use server::{create_router, run_server};
pub use types::*;

