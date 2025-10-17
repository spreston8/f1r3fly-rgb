/// Balance query operations
///
/// Handles Bitcoin and RGB balance queries.
use super::shared::*;
use crate::error::WalletError;
use bitcoin::Network;

/// Get balance for a wallet (Bitcoin + RGB assets)
pub async fn get_balance(
    storage: &Storage,
    balance_checker: &BalanceChecker,
    rgb_runtime_manager: &RgbRuntimeManager,
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

    // Create per-wallet RGB manager
    let rgb_data_dir = storage.base_dir().join(wallet_name).join("rgb_data");
    let rgb_manager = RgbManager::new(rgb_data_dir)?;

    // Check each UTXO for RGB assets
    for utxo in &mut balance.utxos {
        let txid = match utxo.txid.parse::<bitcoin::Txid>() {
            Ok(t) => t,
            Err(_) => continue,
        };

        // Check if UTXO is occupied by RGB assets
        if let Ok(is_occupied) = rgb_manager.check_utxo_occupied(txid, utxo.vout) {
            utxo.is_occupied = is_occupied;

            // If occupied, get the bound assets
            if is_occupied {
                if let Ok(assets) = rgb_manager.get_bound_assets(txid, utxo.vout) {
                    utxo.bound_assets = assets;
                }
            }
        }
    }

    // Get all known contracts from RGB runtime (even with 0 balance)
    // Note: Uses cached state for fast loading. Call sync_rgb_runtime() to update.
    let mut known_contracts = Vec::new();
    if let Ok(runtime) = rgb_runtime_manager.init_runtime_no_sync(wallet_name) {
        use hypersonic::StateName;
        use std::str::FromStr;

        for contract_id in runtime.contracts.contract_ids() {
            // Get contract state and articles
            let state = runtime.contracts.contract_state(contract_id);
            let articles = runtime.contracts.contract_articles(contract_id);

            // Extract ticker from immutable state (same as get_bound_assets)
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

            // Calculate total balance for this contract
            let mut total_balance = 0u64;
            for utxo in &balance.utxos {
                for asset in &utxo.bound_assets {
                    if asset.asset_id == contract_id.to_string() {
                        total_balance += asset.amount.parse::<u64>().unwrap_or(0);
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
    }
    balance.known_contracts = known_contracts;

    Ok(balance)
}
