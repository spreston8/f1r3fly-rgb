/// Wallet Core Module
/// 
/// Modular wallet implementation with clear separation of concerns:
/// 
/// - `manager.rs` - Orchestrator for all wallet operations
/// - `wallet_ops.rs` - Wallet lifecycle operations
/// - `address_ops.rs` - Address management
/// - `balance_ops.rs` - Balance queries
/// - `sync_ops.rs` - Synchronization logic
/// - `bitcoin_ops.rs` - Bitcoin transactions
/// - `rgb_ops_traditional.rs` - Traditional RGB operations (consignment-based)
/// - `rgb_ops_f1r3fly.rs` - F1r3fly RGB operations (state-based)
/// - `rgb_transfer_ops.rs` - RGB invoice & transfer (shared utilities)
/// - `rgb_consignment_ops.rs` - RGB consignment handling (traditional only)
/// - `shared/` - Reusable components (addresses, balance, keys, etc.)

// Operation modules
pub mod address_ops;
pub mod balance_ops;
pub mod bitcoin_ops;
pub mod rgb_consignment_ops;
pub mod rgb_ops_f1r3fly;
pub mod rgb_ops_traditional;
pub mod rgb_transfer_ops;
pub mod sync_ops;
pub mod wallet_ops;

// Shared components (copied from old wallet/)
pub mod shared;

// Main manager (orchestrator)
pub mod manager;

// Re-export the manager as the main entry point
pub use manager::WalletManager;

