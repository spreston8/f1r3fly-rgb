//! Data models for wallet storage

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Metadata {
    pub name: String,
    pub created_at: DateTime<Utc>,
    pub network: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletState {
    pub last_synced_height: Option<u64>,
    pub used_addresses: Vec<u32>,
    pub public_address_index: u32,
    pub internal_next_index: u32,
}

impl Default for WalletState {
    fn default() -> Self {
        Self {
            last_synced_height: None,
            used_addresses: Vec::new(),
            public_address_index: 0,
            internal_next_index: 1,
        }
    }
}

