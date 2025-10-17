/// Synchronization operations
/// 
/// Handles wallet and RGB runtime synchronization with blockchain.

use super::shared::*;
use crate::api::types::SyncResult;
use crate::error::WalletError;
use bitcoin::Network;

/// Sync wallet with blockchain
pub async fn sync_wallet(
    storage: &Storage,
    balance_checker: &BalanceChecker,
    wallet_name: &str,
) -> Result<SyncResult, WalletError> {
    if !storage.wallet_exists(wallet_name) {
        return Err(WalletError::WalletNotFound(wallet_name.to_string()));
    }

    let descriptor = storage.load_descriptor(wallet_name)?;
    let mut state = storage.load_state(wallet_name)?;

    let tip_height = balance_checker.get_tip_height().await?;

    const GAP_LIMIT: u32 = 20;
    let addresses =
        AddressManager::derive_addresses(&descriptor, 0, GAP_LIMIT, Network::Signet)?;

    let mut new_transactions = 0;

    for (index, address) in addresses {
        let utxos = balance_checker
            .get_address_utxos(&address, index)
            .await?;
        if !utxos.is_empty() && !state.used_addresses.contains(&index) {
            state.used_addresses.push(index);
            new_transactions += utxos.len();
        }
    }

    state.last_synced_height = Some(tip_height);
    storage.save_state(wallet_name, &state)?;

    Ok(SyncResult {
        synced_height: tip_height,
        addresses_checked: GAP_LIMIT,
        new_transactions,
    })
}

/// Sync RGB runtime with blockchain (public API)
pub fn sync_rgb_runtime(
    storage: &Storage,
    rgb_runtime_manager: &RgbRuntimeManager,
    wallet_name: &str,
) -> Result<(), WalletError> {
    sync_rgb_internal(storage, rgb_runtime_manager, wallet_name, 1, "Syncing RGB runtime")
}

/// Internal RGB sync method with configurable confirmations
pub(crate) fn sync_rgb_internal(
    storage: &Storage,
    rgb_runtime_manager: &RgbRuntimeManager,
    wallet_name: &str,
    confirmations: u32,
    log_prefix: &str,
) -> Result<(), WalletError> {
    use std::time::Instant;
    
    if !storage.wallet_exists(wallet_name) {
        return Err(WalletError::WalletNotFound(wallet_name.to_string()));
    }

    let mut runtime = rgb_runtime_manager.init_runtime_no_sync(wallet_name)?;

    let conf_str = if confirmations == 1 {
        "1 confirmation".to_string()
    } else {
        format!("{} confirmations", confirmations)
    };
    log::info!("{} ({})", log_prefix, conf_str);
    log::debug!("Starting blockchain scan via Esplora API (this may take 10-15 seconds)...");

    let start = Instant::now();
    runtime.update(confirmations).map_err(|e| {
        log::error!("RGB sync failed after {:?}: {:?}", start.elapsed(), e);
        WalletError::Rgb(format!("RGB sync failed: {:?}", e))
    })?;
    let duration = start.elapsed();

    log::info!("RGB state synced in {:?}", duration);
    if duration.as_secs() > 5 {
        log::warn!("Sync took longer than expected ({:?}). This is due to sequential Esplora API queries.", duration);
    }
    Ok(())
}

/// Sync RGB runtime after a state-changing operation
pub(crate) fn sync_rgb_after_state_change(
    storage: &Storage,
    rgb_runtime_manager: &RgbRuntimeManager,
    wallet_name: &str,
) -> Result<(), WalletError> {
    sync_rgb_internal(storage, rgb_runtime_manager, wallet_name, 1, "Syncing RGB runtime after state change")
}

