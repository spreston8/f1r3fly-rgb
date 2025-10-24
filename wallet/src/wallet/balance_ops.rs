/// Balance query operations
///
/// Handles Bitcoin and RGB balance queries.
use super::shared::*;
use super::shared::rgb::BoundAsset;
use crate::error::WalletError;
use bitcoin::Network;
use std::collections::HashMap;
use bpstd::psbt::PsbtConstructor;

/// Get Bitcoin network from config
fn get_network() -> Network {
    crate::config::WalletConfig::from_env().bitcoin_network
}

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
    let network = get_network();
    let addresses_with_indices =
        AddressManager::derive_addresses(&descriptor, 0, GAP_LIMIT, network)?;

    // Async HTTP calls to Esplora
    let mut balance = balance_checker
        .calculate_balance(&addresses_with_indices)
        .await?;
    
    // Set display address to public address (index 0)
    let public_address = AddressManager::derive_address(&descriptor, state.public_address_index, network)?;
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
    rgb_runtime_manager: &RgbRuntimeManager,
    wallet_name: &str,
    utxos: &[balance::UTXO],
) -> Result<RgbBalanceData, WalletError> {
    use hypersonic::StateName;
    use std::str::FromStr;
    use bpstd::{Outpoint, Vout};
    use bitcoin::hashes::Hash;
    use amplify::ByteArray;

    // Create ephemeral runtime (matches RGB CLI)
    log::debug!("Creating ephemeral RGB runtime for balance query");
    let mut runtime = rgb_runtime_manager.init_runtime_no_sync(wallet_name)?;

    {
        let mut utxo_assets: HashMap<String, Vec<BoundAsset>> = HashMap::new();
        let mut known_contracts = Vec::new();

    // Sync RGB state for balance query (UTXOs + witnesses)
    log::debug!("Syncing RGB state for balance query...");
    if let Err(e) = runtime.update(1) {
        log::warn!("Failed to sync RGB state for balance query: {:?}", e);
        // Continue anyway - show balance with cached state
    } else {
        log::debug!("RGB state synced successfully");
    }
    
    // 🔍 DEBUG: Log wallet's UTXO set for troubleshooting
    use rgbp::{OwnerProvider, UtxoSet};
    let utxo_count = runtime.wallet.utxos().len();
    log::debug!("Wallet has {} UTXO(s) in internal set after sync", utxo_count);

    // Debug: Show registered seal count  
    let seal_count = runtime.wallet.descriptor().seals().count();
    log::debug!("Checking balance with {} registered seal(s)", seal_count);
    
    // 🔍 DEBUG: Check if contracts have any data
    let contract_count = runtime.contracts.contract_ids().count();
    log::debug!("Found {} contract(s) in stockpile", contract_count);
        
        // 🔍 DEBUG: Check witness statuses for each contract
        for contract_id in runtime.contracts.contract_ids() {
            let witnesses: Vec<_> = runtime.contracts.contract_witnesses(contract_id).collect();
            log::debug!("Contract {} has {} witness(es)", contract_id, witnesses.len());
            for (idx, witness) in witnesses.iter().enumerate() {
                log::debug!("  Witness {}: id={}, status={:?}, ops={}", 
                    idx, witness.id, witness.status, witness.opids.len());
            }
        }

        // 🔍 DEBUG: Check contract state details
        for contract_id in runtime.contracts.contract_ids() {
            let state = runtime.contracts.contract_state(contract_id);
            log::debug!("Contract {} state details:", contract_id);
            log::debug!("  Owned state count: {}", state.owned.len());
            for (assignment_id, token_allocs) in &state.owned {
                log::debug!("    Assignment {}: {} allocation(s)", assignment_id, token_allocs.len());
                for (idx, alloc) in token_allocs.iter().enumerate().take(5) {
                    log::debug!("      Alloc {}: {:?}", idx, alloc);
                }
            }
        }

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
        // ⚠️ CRITICAL: Iterate over RGB allocations (not Bitcoin UTXOs)
        // RGB tracks allocations independently, and the wallet might not have all UTXOs indexed yet
        let mut total_balance = 0u64;

        // Check each RGB allocation to see if we own it  
        for (_state_name, owned_states) in &state.owned {
            for owned_state in owned_states {
                let seal_outpoint = owned_state.assignment.seal.primary;
                
                // Check if this wallet owns this UTXO
                // OwnerProvider trait gives us direct access to check UTXO ownership
                let owns_utxo = {
                    use rgbp::OwnerProvider;
                    // Try to get UTXO info - if it exists, we own it
                    runtime.wallet.utxo(seal_outpoint).is_some()
                };
                
                if owns_utxo {
                    // Extract amount from the state data (StrictVal)
                    if let Ok(amount) = owned_state.assignment.data.to_string().parse::<u64>() {
                        total_balance += amount;

                        // Add to UTXO assets map (convert outpoint back to string)
                        let key = format!("{}:{}", seal_outpoint.txid, seal_outpoint.vout.to_u32());
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
    }
    // Runtime drops here → FileHolder::drop() auto-saves to disk
}

/// RGB balance data structure
#[derive(Debug)]
pub struct RgbBalanceData {
    pub utxo_assets: HashMap<String, Vec<BoundAsset>>,
    pub known_contracts: Vec<balance::KnownContract>,
}
