/// Balance query operations
///
/// Handles Bitcoin and RGB balance queries.
use super::shared::*;
use super::shared::rgb::BoundAsset;
use crate::error::WalletError;
use bitcoin::Network;
use std::collections::HashMap;
use bpstd::psbt::PsbtConstructor;

/// Get Bitcoin balance only (async HTTP calls)
pub async fn get_bitcoin_balance(
    storage: &Storage,
    balance_checker: &BalanceChecker,
    wallet_name: &str,
) -> Result<balance::BalanceInfo, WalletError> {
    if !storage.wallet_exists(wallet_name) {
        return Err(WalletError::WalletNotFound(wallet_name.to_string()));
    }

    let descriptor = storage.load_descriptor(wallet_name)?;
    let state = storage.load_state(wallet_name)?;

    const GAP_LIMIT: u32 = 20;
    let addresses_with_indices =
        AddressManager::derive_addresses(&descriptor, 0, GAP_LIMIT, Network::Signet)?;

    // Async HTTP calls to Esplora
    let mut balance = balance_checker
        .calculate_balance(&addresses_with_indices)
        .await?;
    
    // Set display address to public address (index 0)
    let public_address = AddressManager::derive_address(&descriptor, state.public_address_index, Network::Signet)?;
    balance.display_address = public_address.to_string();
    
    log::debug!(
        "Balance aggregated from {} addresses, displaying public address: {}", 
        GAP_LIMIT, 
        balance.display_address
    );

    Ok(balance)
}

/// Get RGB balance only (sync, blocking)
pub fn get_rgb_balance_sync(
    _storage: &Storage,
    rgb_runtime_cache: &std::sync::Arc<super::shared::RgbRuntimeCache>,
    wallet_name: &str,
    utxos: &[balance::UTXO],
) -> Result<RgbBalanceData, WalletError> {
    use hypersonic::StateName;
    use std::str::FromStr;
    use bpstd::{Outpoint, Vout};
    use bitcoin::hashes::Hash;
    use amplify::ByteArray;

    // Get cached runtime and query balance (Phase 2)
    log::debug!("Acquiring cached RGB runtime for balance query");
    let guard = rgb_runtime_cache.get_or_create(wallet_name)?;

    guard.execute(|runtime| {
        let mut utxo_assets: HashMap<String, Vec<BoundAsset>> = HashMap::new();
        let mut known_contracts = Vec::new();

        // Sync witness confirmations from blockchain
        log::debug!("Syncing RGB witness confirmations for balance query...");
        if let Err(e) = runtime.update(1) {
            log::warn!("Failed to sync RGB witness state for balance query: {:?}", e);
            // Continue anyway - show balance with cached witness state
        } else {
            log::debug!("RGB witness state synced successfully");
        }

        // Debug: Show registered seal count
        let seal_count = runtime.wallet.descriptor().seals().count();
        log::debug!("Checking balance with {} registered seal(s)", seal_count);

        // Now check UTXOs for bound assets using the SYNCED runtime state
        for contract_id in runtime.contracts.contract_ids() {
        let state = runtime.contracts.contract_state(contract_id);
        let articles = runtime.contracts.contract_articles(contract_id);

        // Extract ticker from immutable state
        let ticker = StateName::from_str("ticker")
            .ok()
            .and_then(|name| state.immutable.get(&name))
            .and_then(|states| states.first())
            .map(|s| s.data.verified.to_string())
            .unwrap_or_else(|| "N/A".to_string());

        // Extract name from immutable state or fallback to articles
        let asset_name = StateName::from_str("name")
            .ok()
            .and_then(|name| state.immutable.get(&name))
            .and_then(|states| states.first())
            .map(|s| s.data.verified.to_string())
            .unwrap_or_else(|| articles.issue().meta.name.to_string());

        // Calculate balance from synced contract state
        let mut total_balance = 0u64;

        // Check each UTXO against this contract's owned state
        for utxo in utxos {
            let txid = match utxo.txid.parse::<bitcoin::Txid>() {
                Ok(t) => t,
                Err(_) => continue,
            };
            
            // Convert bitcoin::Txid to bpstd::Txid via byte array
            let txid_bytes: [u8; 32] = txid.to_byte_array();
            let bp_txid = bpstd::Txid::from_byte_array(txid_bytes);
            let target_outpoint = Outpoint::new(bp_txid, Vout::from_u32(utxo.vout));

            // Check if this UTXO has assets from this contract
            for (_state_name, owned_states) in &state.owned {
                for owned_state in owned_states {
                    let seal_outpoint = owned_state.assignment.seal.primary;
                    if seal_outpoint == target_outpoint {
                        // Extract amount from the state data (StrictVal)
                        if let Ok(amount) = owned_state.assignment.data.to_string().parse::<u64>() {
                            total_balance += amount;

                            // Add to UTXO assets map
                            let key = format!("{}:{}", utxo.txid, utxo.vout);
                            utxo_assets.entry(key).or_insert_with(Vec::new).push(BoundAsset {
                                asset_id: contract_id.to_string(),
                                ticker: ticker.clone(),
                                asset_name: asset_name.clone(),
                                amount: amount.to_string(),
                            });
                        }
                    }
                }
            }
        }

        known_contracts.push(balance::KnownContract {
            contract_id: contract_id.to_string(),
            ticker,
            name: asset_name,
            balance: total_balance,
        });
        }

        Ok(RgbBalanceData {
            utxo_assets,
            known_contracts,
        })
    })
}

/// RGB balance data structure
#[derive(Debug)]
pub struct RgbBalanceData {
    pub utxo_assets: HashMap<String, Vec<BoundAsset>>,
    pub known_contracts: Vec<balance::KnownContract>,
}
