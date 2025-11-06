//! Bitcoin protocol operations
//! 
//! - UTXO management
//! - Bitcoin sending
//! - Transaction building and signing
//! - Balance queries

pub mod transaction;
pub mod signer;
pub mod utxo;
pub mod send;
pub mod balance;
pub mod balance_checker;
pub mod network;

// Re-export main types
pub use transaction::{TransactionBuilder, broadcast_transaction};
pub use signer::WalletSigner;
pub use utxo::{create_utxo, unlock_utxo};
pub use send::send_bitcoin;
pub use network::get_network;
pub use balance_checker::{BalanceChecker, BalanceInfo, UTXO};

