//! Bitcoin network utility

use bitcoin::Network;

/// Get the configured Bitcoin network from environment
pub fn get_network() -> Network {
    crate::config::WalletConfig::from_env().bitcoin_network
}

