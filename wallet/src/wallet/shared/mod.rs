/// Shared wallet components used across operations modules
/// 
/// Core wallet building blocks shared across all operation modules.

pub mod addresses;
pub mod balance;
pub mod keys;
pub mod rgb;
pub mod rgb_runtime;
pub mod signer;
pub mod storage;
pub mod transaction;

// Re-export commonly used items for convenience
pub use addresses::AddressManager;
pub use balance::BalanceChecker;
pub use keys::KeyManager;
pub use rgb::RgbManager;
pub use rgb_runtime::RgbRuntimeManager;
pub use signer::WalletSigner;
pub use storage::{Metadata, Storage, WalletState};

