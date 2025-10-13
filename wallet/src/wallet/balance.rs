use bitcoin::Address;
use serde::{Deserialize, Serialize};

pub struct BalanceChecker {
    client: reqwest::Client,
    base_url: String,
}

impl BalanceChecker {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: "https://mempool.space/signet/api".to_string(),
        }
    }

    pub async fn get_address_utxos(
        &self,
        address: &Address,
        address_index: u32,
    ) -> Result<Vec<UTXO>, crate::error::WalletError> {
        let url = format!("{}/address/{}/utxo", self.base_url, address);
        
        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| crate::error::WalletError::Esplora(e.to_string()))?;

        if !response.status().is_success() {
            return Ok(Vec::new());
        }

        let utxo_list: Vec<serde_json::Value> = response
            .json()
            .await
            .map_err(|e| crate::error::WalletError::Esplora(e.to_string()))?;

        let tip_height = self.get_tip_height().await.unwrap_or(0);

        let utxos = utxo_list
            .iter()
            .filter_map(|utxo| {
                let txid = utxo["txid"].as_str()?.to_string();
                let vout = utxo["vout"].as_u64()? as u32;
                let amount_sats = utxo["value"].as_u64()?;
                let confirmed = utxo["status"]["confirmed"].as_bool().unwrap_or(false);
                let block_height = utxo["status"]["block_height"].as_u64().unwrap_or(0);

                let confirmations = if confirmed && tip_height > 0 && block_height > 0 {
                    (tip_height.saturating_sub(block_height) + 1) as u32
                } else {
                    0
                };

                Some(UTXO {
                    txid,
                    vout,
                    amount_sats,
                    address: address.to_string(),
                    address_index,
                    confirmations,
                    is_occupied: false,
                    bound_assets: Vec::new(),
                })
            })
            .collect();

        Ok(utxos)
    }

    pub async fn get_tip_height(&self) -> Result<u64, crate::error::WalletError> {
        let url = format!("{}/blocks/tip/height", self.base_url);

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| crate::error::WalletError::Esplora(e.to_string()))?;

        let height: u64 = response
            .text()
            .await
            .map_err(|e| crate::error::WalletError::Esplora(e.to_string()))?
            .parse()
            .map_err(|e: std::num::ParseIntError| crate::error::WalletError::Esplora(e.to_string()))?;

        Ok(height)
    }

    pub async fn calculate_balance(
        &self,
        addresses_with_indices: &[(u32, Address)],
    ) -> Result<BalanceInfo, crate::error::WalletError> {
        let futures: Vec<_> = addresses_with_indices
            .iter()
            .map(|(_, address)| {
                let client = self.client.clone();
                let base_url = self.base_url.clone();
                let address_str = address.to_string();
                async move {
                    let url = format!("{}/address/{}", base_url, address_str);

                    match client.get(&url).send().await {
                        Ok(response) => {
                            if response.status().is_success() {
                                if let Ok(addr_info) = response.json::<serde_json::Value>().await {
                                    let confirmed_funded = addr_info["chain_stats"]["funded_txo_sum"]
                                        .as_u64()
                                        .unwrap_or(0);
                                    let confirmed_spent = addr_info["chain_stats"]["spent_txo_sum"]
                                        .as_u64()
                                        .unwrap_or(0);
                                    let confirmed_balance =
                                        confirmed_funded.saturating_sub(confirmed_spent);

                                    let unconfirmed_funded = addr_info["mempool_stats"]
                                        ["funded_txo_sum"]
                                        .as_u64()
                                        .unwrap_or(0);
                                    let unconfirmed_spent = addr_info["mempool_stats"]
                                        ["spent_txo_sum"]
                                        .as_u64()
                                        .unwrap_or(0);
                                    let unconfirmed_balance =
                                        unconfirmed_funded.saturating_sub(unconfirmed_spent);

                                    return (confirmed_balance, unconfirmed_balance);
                                }
                            }
                        }
                        Err(_) => {}
                    }
                    (0, 0)
                }
            })
            .collect();

        let results = futures::future::join_all(futures).await;

        let confirmed_sats: u64 = results.iter().map(|(confirmed, _)| confirmed).sum();
        let unconfirmed_sats: u64 = results.iter().map(|(_, unconfirmed)| unconfirmed).sum();

        let mut all_utxos = Vec::new();
        for (index, address) in addresses_with_indices {
            if let Ok(utxos) = self.get_address_utxos(address, *index).await {
                all_utxos.extend(utxos);
            }
        }

        Ok(BalanceInfo {
            confirmed_sats,
            unconfirmed_sats,
            utxo_count: all_utxos.len(),
            utxos: all_utxos,
            known_contracts: Vec::new(),
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UTXO {
    pub txid: String,
    pub vout: u32,
    pub amount_sats: u64,
    pub address: String,
    pub address_index: u32,
    pub confirmations: u32,
    /// Indicates if this UTXO has RGB assets bound to it
    pub is_occupied: bool,
    /// RGB assets bound to this UTXO (empty if not occupied)
    pub bound_assets: Vec<super::rgb::BoundAsset>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnownContract {
    pub contract_id: String,
    pub ticker: String,
    pub name: String,
    pub balance: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BalanceInfo {
    pub confirmed_sats: u64,
    pub unconfirmed_sats: u64,
    pub utxo_count: usize,
    pub utxos: Vec<UTXO>,
    pub known_contracts: Vec<KnownContract>,
}

