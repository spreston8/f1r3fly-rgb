//! RGB protocol operations
//! 
//! - Asset issuance
//! - Invoice generation
//! - Transfer operations
//! - Consignment handling
//! - Runtime management

pub mod runtime;
pub mod cache;
pub mod lifecycle;
pub mod asset;
pub mod transfer;
pub mod consignment;

// Re-export main types
pub use runtime::RgbRuntimeManager;
pub use cache::{RgbRuntimeCache, RuntimeGuard, CacheStats};
pub use lifecycle::{RuntimeLifecycleManager, LifecycleConfig};
pub use asset::{RgbManager, IssueAssetRequest, IssueAssetResponse, BoundAsset, RgbAssignment, RgbCreateParams, RgbIssuer, map_precision};
pub use transfer::{generate_rgb_invoice_sync, find_utxo_for_selection, send_transfer};
pub use consignment::{accept_consignment, export_genesis_consignment};

