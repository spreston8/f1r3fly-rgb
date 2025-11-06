//! Core wallet operations
//! 
//! - Wallet lifecycle (create, import, delete)
//! - Address management
//! - Blockchain synchronization

pub mod lifecycle;
pub mod addresses;
pub mod sync;
pub mod address_manager;

// Public API exports
pub use lifecycle::{create_wallet, import_wallet, list_wallets, delete_wallet};
pub use addresses::{get_addresses, get_primary_address};
pub use sync::{sync_wallet, sync_rgb_runtime, sync_rgb_internal, sync_rgb_after_state_change};
pub use address_manager::AddressManager;

