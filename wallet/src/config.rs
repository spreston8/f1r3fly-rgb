/// Wallet configuration from environment variables
/// 
/// Controls Bitcoin network type and Esplora API endpoint.
/// Defaults to Signet for production compatibility.

use std::env;

#[derive(Clone, Debug)]
pub struct WalletConfig {
    /// Bitcoin network type (for bitcoin crate)
    pub bitcoin_network: bitcoin::Network,
    /// BP-Std network type (for RGB/BP operations)
    pub bpstd_network: bpstd::Network,
    /// Esplora API base URL
    pub esplora_url: String,
    /// Optional Bitcoin Core RPC URL (for direct RPC access)
    pub bitcoin_rpc_url: Option<String>,
    /// Public URL of this wallet API server (for generating download links)
    pub public_url: String,
}

impl WalletConfig {
    /// Load configuration from environment variables
    /// 
    /// Environment variables:
    /// - `BITCOIN_NETWORK`: "signet" (default) or "regtest"
    /// - `ESPLORA_URL`: Esplora API endpoint (optional, has sensible defaults)
    /// - `BITCOIN_RPC_URL`: Bitcoin Core RPC endpoint (optional)
    /// - `PUBLIC_URL`: Public URL of this wallet API (for generating download links)
    /// 
    /// # Examples
    /// 
    /// ```bash
    /// # Use Signet (default)
    /// cargo run
    /// 
    /// # Use Regtest with local Esplora mock
    /// BITCOIN_NETWORK=regtest ESPLORA_URL=http://localhost:3000 cargo run
    /// ```
    pub fn from_env() -> Self {
        let network_str = env::var("BITCOIN_NETWORK")
            .unwrap_or_else(|_| "signet".to_string())
            .to_lowercase();
        
        let (bitcoin_network, bpstd_network) = match network_str.as_str() {
            "regtest" => {
                log::info!("🔧 Using REGTEST network");
                (
                    bitcoin::Network::Regtest,
                    bpstd::Network::Regtest,
                )
            }
            "signet" | "" => {
                log::info!("🌐 Using SIGNET network");
                (
                    bitcoin::Network::Signet,
                    bpstd::Network::Signet,
                )
            }
            other => {
                log::warn!("⚠️  Unknown network '{}', defaulting to Signet", other);
                (
                    bitcoin::Network::Signet,
                    bpstd::Network::Signet,
                )
            }
        };
        
        // Determine Esplora URL
        let esplora_url = env::var("ESPLORA_URL")
            .unwrap_or_else(|_| {
                let default_url = match network_str.as_str() {
                    "regtest" => {
                        log::info!("📡 Esplora URL: http://localhost:3000 (Regtest default)");
                        "http://localhost:3000".to_string()
                    }
                    _ => {
                        log::info!("📡 Esplora URL: https://mempool.space/signet/api");
                        "https://mempool.space/signet/api".to_string()
                    }
                };
                default_url
            });
        
        // Optional Bitcoin Core RPC URL
        let bitcoin_rpc_url = env::var("BITCOIN_RPC_URL").ok();
        if let Some(ref url) = bitcoin_rpc_url {
            log::info!("🔗 Bitcoin RPC URL: {}", url);
        }
        
        // Public URL for this wallet API server (used for download links)
        let public_url = env::var("PUBLIC_URL")
            .unwrap_or_else(|_| "http://localhost:3000".to_string());
        log::info!("🌍 Public API URL: {}", public_url);
        
        Self {
            bitcoin_network,
            bpstd_network,
            esplora_url,
            bitcoin_rpc_url,
            public_url,
        }
    }
    
    /// Get the BIP44 coin type for this network
    /// 
    /// - Mainnet: 0
    /// - Testnet/Signet/Regtest: 1
    pub fn coin_type(&self) -> u32 {
        match self.bitcoin_network {
            bitcoin::Network::Bitcoin => 0,
            _ => 1, // All test networks use coin type 1
        }
    }
    
    /// Get the derivation path for this network
    /// 
    /// Returns: "m/84'/1'/0'" for test networks, "m/84'/0'/0'" for mainnet
    pub fn derivation_path(&self) -> String {
        format!("m/84'/{}'/'0'", self.coin_type())
    }
}

impl Default for WalletConfig {
    /// Default configuration (Signet)
    fn default() -> Self {
        Self {
            bitcoin_network: bitcoin::Network::Signet,
            bpstd_network: bpstd::Network::Signet,
            esplora_url: "https://mempool.space/signet/api".to_string(),
            bitcoin_rpc_url: None,
            public_url: "http://localhost:3000".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_default_is_signet() {
        let config = WalletConfig::default();
        assert!(matches!(config.bitcoin_network, bitcoin::Network::Signet));
        assert!(matches!(config.bpstd_network, bpstd::Network::Signet));
    }
    
    #[test]
    fn test_coin_type() {
        let signet_config = WalletConfig {
            bitcoin_network: bitcoin::Network::Signet,
            ..Default::default()
        };
        assert_eq!(signet_config.coin_type(), 1);
        
        let regtest_config = WalletConfig {
            bitcoin_network: bitcoin::Network::Regtest,
            ..Default::default()
        };
        assert_eq!(regtest_config.coin_type(), 1);
    }
}

