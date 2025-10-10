use std::fs;
use std::path::PathBuf;
use std::str::FromStr;

use amplify::ByteArray;
use bitcoin::hashes::Hash;
use bpstd::seals::TxoSeal;
use bpstd::{Network, Outpoint, Vout};
use hypersonic::StateName;
use rgb::{Consensus, Contracts};
use rgb_persist_fs::StockpileDir;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoundAsset {
    pub asset_id: String,
    pub asset_name: String,
    pub ticker: String,
    pub amount: String,
}

pub struct RgbManager {
    data_dir: PathBuf,
    network: Network,
}

impl RgbManager {
    pub fn new(data_dir: PathBuf, network: Network) -> Result<Self, crate::error::WalletError> {
        fs::create_dir_all(&data_dir)
            .map_err(|e| crate::error::WalletError::Rgb(format!("Failed to create RGB data directory: {}", e)))?;
        
        Ok(Self { data_dir, network })
    }

    fn load_contracts(&self) -> Result<Contracts<StockpileDir<TxoSeal>>, crate::error::WalletError> {
        let stockpile = StockpileDir::load(self.data_dir.clone(), Consensus::Bitcoin, true)
            .map_err(|e| crate::error::WalletError::Rgb(format!("Failed to load RGB stockpile: {}", e)))?;
        
        Ok(Contracts::load(stockpile))
    }

    pub fn check_utxo_occupied(
        &self,
        txid: bitcoin::Txid,
        vout: u32,
    ) -> Result<bool, crate::error::WalletError> {
        let contracts = match self.load_contracts() {
            Ok(c) => c,
            Err(_) => return Ok(false),
        };

        let bp_txid = bpstd::Txid::from_byte_array(txid.to_byte_array());
        let target_outpoint = Outpoint::new(bp_txid, Vout::from_u32(vout));

        for contract_id in contracts.contract_ids() {
            let state = contracts.contract_state(contract_id);

            for (_state_name, owned_states) in state.owned {
                for owned_state in owned_states {
                    // TxoSeal contains an outpoint - extract it for comparison
                    let seal_outpoint = owned_state.assignment.seal.primary;
                    if seal_outpoint == target_outpoint {
                        return Ok(true);
                    }
                }
            }
        }

        Ok(false)
    }

    pub fn get_bound_assets(
        &self,
        txid: bitcoin::Txid,
        vout: u32,
    ) -> Result<Vec<BoundAsset>, crate::error::WalletError> {
        let contracts = self.load_contracts()?;
        
        let bp_txid = bpstd::Txid::from_byte_array(txid.to_byte_array());
        let target_outpoint = Outpoint::new(bp_txid, Vout::from_u32(vout));
        
        let mut assets = Vec::new();

        for contract_id in contracts.contract_ids() {
            let state = contracts.contract_state(contract_id);
            let articles = contracts.contract_articles(contract_id);

            for (_state_name, owned_states) in state.owned {
                for owned_state in owned_states {
                    let seal_outpoint = owned_state.assignment.seal.primary;
                    if seal_outpoint == target_outpoint {
                        let ticker = state.immutable
                            .get(&StateName::from_str("ticker").map_err(|e| {
                                crate::error::WalletError::Rgb(format!("Invalid state name: {}", e))
                            })?)
                            .and_then(|states| states.first())
                            .map(|s| s.data.verified.to_string())
                            .unwrap_or_else(|| "N/A".to_string());

                        let asset_name = state.immutable
                            .get(&StateName::from_str("name").map_err(|e| {
                                crate::error::WalletError::Rgb(format!("Invalid state name: {}", e))
                            })?)
                            .and_then(|states| states.first())
                            .map(|s| s.data.verified.to_string())
                            .unwrap_or_else(|| articles.issue().meta.name.to_string());

                        let amount = owned_state.assignment.data.to_string();

                        assets.push(BoundAsset {
                            asset_id: contract_id.to_string(),
                            asset_name,
                            ticker,
                            amount,
                        });
                    }
                }
            }
        }

        Ok(assets)
    }
}

