/// Common test utilities for RGB wallet integration tests
/// 
/// This module provides shared test infrastructure including:
/// - Network configuration (Signet/Regtest)
/// - Test environment setup and cleanup
/// - Confirmation waiting (with Regtest mining support)
/// - Helper functions for transaction queries

use std::time::Instant;
use tempfile::TempDir;
use wallet::manager::WalletManager;
use wallet::storage::Storage;
use wallet::bitcoin::balance_checker::BalanceInfo;

/// Test environment with automatic cleanup
pub struct TestEnvironment {
    pub temp_dir: TempDir,
    pub manager: WalletManager,
    pub wallet1_name: String,
    pub wallet2_name: String,
}

impl TestEnvironment {
    pub fn new(test_name: &str) -> anyhow::Result<Self> {
        // Create temp directory
        let temp_dir = TempDir::new()?;
        log::info!("📁 Test directory: {:?}", temp_dir.path());
        
        // Create storage with temp path
        let storage = Storage::new_with_base_dir(temp_dir.path().to_path_buf());
        
        // Create WalletManager
        let manager = WalletManager::new_with_storage(storage);
        
        // Generate unique wallet names with timestamp
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        Ok(Self {
            temp_dir,
            manager,
            wallet1_name: format!("test-sender-{}-{}", test_name, timestamp),
            wallet2_name: format!("test-recipient-{}-{}", test_name, timestamp),
        })
    }
}

impl Drop for TestEnvironment {
    fn drop(&mut self) {
        log::info!("\n🧹 Cleaning up test environment...");
        
        // Delete wallets through manager
        if let Err(e) = self.manager.delete_wallet(&self.wallet1_name) {
            log::warn!("Failed to delete wallet1: {}", e);
        }
        
        if let Err(e) = self.manager.delete_wallet(&self.wallet2_name) {
            log::warn!("Failed to delete wallet2: {}", e);
        }
        
        log::info!("✓ Cleanup complete (temp dir will auto-remove)");
    }
}

/// Network configuration for tests
#[derive(Debug, Clone)]
pub struct TestNetworkConfig {
    pub esplora_url: String,
    pub is_regtest: bool,
}

impl TestNetworkConfig {
    pub fn from_env() -> Self {
        let network = std::env::var("BITCOIN_NETWORK")
            .unwrap_or_else(|_| "signet".to_string())
            .to_lowercase();
        
        let esplora_url = std::env::var("ESPLORA_URL")
            .unwrap_or_else(|_| {
                if network == "regtest" {
                    "http://localhost:3000".to_string()
                } else {
                    "https://mempool.space/signet/api".to_string()
                }
            });
        
        let is_regtest = network == "regtest";
        
        if is_regtest {
            log::info!("🔧 Test Mode: REGTEST (fast local testing)");
            log::info!("   Esplora Mock: {}", esplora_url);
        } else {
            log::info!("🌐 Test Mode: SIGNET (real network)");
            log::info!("   Esplora API: {}", esplora_url);
        }
        
        Self {
            esplora_url,
            is_regtest,
        }
    }
}

/// Mine a block in Regtest mode
/// 
/// Tries two methods in order:
/// 1. Call esplora-mock's /regtest/mine endpoint (if available)
/// 2. Fall back to bitcoin-cli (for use with Electrs)
pub async fn mine_regtest_block(esplora_url: &str) -> anyhow::Result<u64> {
    // Try esplora-mock endpoint first
    let url = format!("{}/regtest/mine", esplora_url);
    
    let client = reqwest::Client::new();
    let response = client
        .post(&url)
        .json(&serde_json::json!({ "count": 1 }))
        .send()
        .await;
    
    match response {
        Ok(resp) if resp.status().is_success() => {
            // esplora-mock endpoint worked
            let result: serde_json::Value = resp.json().await?;
            let new_height = result["new_height"].as_u64()
                .ok_or_else(|| anyhow::anyhow!("Invalid response from mining endpoint"))?;
            log::debug!("⛏️  Mined block via esplora-mock, new height: {}", new_height);
            Ok(new_height)
        }
        _ => {
            // Fall back to bitcoin-cli (for Electrs setup)
            // Note: Mining logs are at trace level to avoid spam when mining 100+ blocks
            
            // Default to project root .bitcoin if BITCOIN_DATADIR not set
            let datadir = std::env::var("BITCOIN_DATADIR")
                .unwrap_or_else(|_| {
                    // Try to find project root by going up from current dir
                    let current_dir = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
                    let project_root = current_dir.parent().unwrap_or(&current_dir);
                    format!("{}/.bitcoin", project_root.display())
                });
            
            // Mine to a different address (not the test wallet) to avoid creating new coinbase UTXOs
            // that would need their own 100-block maturity period.
            // Using a valid Regtest address (P2WPKH from a known test vector)
            let mining_address = "bcrt1qw508d6qejxtdg4y5r3zarvary0c5xw7kygt080";
            
            let output = tokio::process::Command::new("bitcoin-cli")
                .args(&[
                    "-regtest",
                    &format!("-datadir={}", datadir),
                    "generatetoaddress",
                    "1",
                    mining_address,
                ])
                .output()
                .await?;
            
            if !output.status.success() {
                anyhow::bail!(
                    "bitcoin-cli failed: {}",
                    String::from_utf8_lossy(&output.stderr)
                );
            }
            
            // Get current height
            let height_output = tokio::process::Command::new("bitcoin-cli")
                .args(&[
                    "-regtest",
                    &format!("-datadir={}", datadir),
                    "getblockcount",
                ])
                .output()
                .await?;
            
            let height_str = String::from_utf8_lossy(&height_output.stdout);
            let height = height_str.trim().parse::<u64>()?;
            
            // Use trace level to avoid spam when mining 100 blocks
            log::trace!("⛏️  Mined block via bitcoin-cli, new height: {}", height);
            Ok(height)
        }
    }
}

/// Wait for transaction confirmation (network-aware)
/// 
/// - Regtest: Mines a block and verifies confirmation
/// - Signet: Polls Esplora API until confirmed
pub async fn wait_for_confirmation(
    txid: &str,
    config: &TestNetworkConfig,
    timeout_secs: u64,
) -> anyhow::Result<u64> {
    if config.is_regtest {
        // Regtest: Mine a block immediately
        log::info!("⛏️  Mining block to confirm transaction...");
        let new_height = mine_regtest_block(&config.esplora_url).await?;
        
        // Wait for Electrs to index the block (with retry logic)
        let max_retries = 10;
        let mut delay_ms = 100;
        
        for attempt in 1..=max_retries {
            tokio::time::sleep(tokio::time::Duration::from_millis(delay_ms)).await;
            
            let url = format!("{}/tx/{}", config.esplora_url, txid);
            if let Ok(resp) = reqwest::get(&url).await {
                if let Ok(json) = resp.json::<serde_json::Value>().await {
                    if json["status"]["confirmed"].as_bool().unwrap_or(false) {
                        let height = json["status"]["block_height"].as_u64().unwrap_or(new_height);
                        log::info!("✅ Confirmed in block {} after {}ms (Regtest)", height, delay_ms * attempt);
                        return Ok(height);
                    }
                }
            }
            
            if attempt < max_retries {
                log::trace!("Transaction not yet confirmed, retrying in {}ms...", delay_ms);
                delay_ms = (delay_ms as f64 * 1.5) as u64; // Exponential backoff
            }
        }
        
        anyhow::bail!("Transaction not confirmed after mining block (tried {} times)", max_retries);
    } else {
        // Signet: Poll for confirmation
        let start = Instant::now();
        let poll_interval = 15; // Poll every 15 seconds
        let attempts = timeout_secs / poll_interval;
        
        log::info!("⏳ Waiting for confirmation (timeout: {}s, polling every {}s)...", 
            timeout_secs, poll_interval);
        
        for attempt in 1..=attempts {
            if start.elapsed().as_secs() > timeout_secs {
                anyhow::bail!("Timeout waiting for confirmation after {} seconds", timeout_secs);
            }
            
            let url = format!("{}/tx/{}", config.esplora_url, txid);
            
            match reqwest::get(&url).await {
                Ok(resp) => {
                    if let Ok(json) = resp.json::<serde_json::Value>().await {
                        if json["status"]["confirmed"].as_bool().unwrap_or(false) {
                            let height = json["status"]["block_height"].as_u64().unwrap();
                            let confirmations = json["status"]["block_height"].as_u64().unwrap_or(1);
                            log::info!("✅ Confirmed in block {} ({} confirmations)", 
                                height, confirmations);
                            return Ok(height);
                        }
                        
                        log::debug!("Transaction status: {:?}", json["status"]);
                    }
                }
                Err(e) => {
                    log::warn!("Esplora API error: {} (will retry)", e);
                }
            }
            
            log::info!("⏳ Not confirmed yet (attempt {}/{}, {}s elapsed)", 
                attempt, attempts, start.elapsed().as_secs());
            tokio::time::sleep(tokio::time::Duration::from_secs(poll_interval)).await;
        }
        
        anyhow::bail!("Transaction not confirmed after {} seconds", timeout_secs)
    }
}

/// Wait for Electrs to index up to a specific block height
/// This ensures Electrs has fully indexed the block before querying wallet state
pub async fn wait_for_electrs_sync(esplora_url: &str, target_height: u64) -> anyhow::Result<()> {
    let max_retries = 20;
    let retry_delay_ms = 500; // 500ms between attempts
    
    for attempt in 1..=max_retries {
        let url = format!("{}/blocks/tip/height", esplora_url);
        
        match reqwest::get(&url).await {
            Ok(resp) => {
                if let Ok(current_height) = resp.text().await?.parse::<u64>() {
                    if current_height >= target_height {
                        log::debug!("✓ Electrs indexed up to block {}", current_height);
                        return Ok(());
                    }
                    log::trace!("Electrs at height {}, waiting for {} (attempt {}/{})", 
                        current_height, target_height, attempt, max_retries);
                }
            }
            Err(e) => {
                log::warn!("Electrs query failed: {} (attempt {}/{})", e, attempt, max_retries);
            }
        }
        
        if attempt < max_retries {
            tokio::time::sleep(tokio::time::Duration::from_millis(retry_delay_ms)).await;
        }
    }
    
    anyhow::bail!("Electrs did not catch up to block {} after {} attempts", 
        target_height, max_retries)
}

/// Additional delay after mining to ensure RGB state is fully synced
/// Use this before querying RGB balance after any blockchain operation
pub async fn ensure_rgb_sync_delay(is_regtest: bool) {
    if is_regtest {
        // Regtest needs time for Electrs to index + RGB to process witnesses
        tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;
        log::trace!("✓ RGB sync delay completed");
    }
    // Signet doesn't need this - confirmations are slow enough
}

/// Pretty-print RGB balance for debugging
pub fn print_rgb_balance(label: &str, balance: &BalanceInfo) {
    log::info!("📊 {} Balance:", label);
    log::info!("  Bitcoin (confirmed): {} sats", balance.confirmed_sats);
    log::info!("  Bitcoin (unconfirmed): {} sats", balance.unconfirmed_sats);
    log::info!("  UTXOs: {}", balance.utxo_count);
    log::info!("  RGB Assets: {}", balance.known_contracts.len());
    
    for contract in &balance.known_contracts {
        log::info!("    - {} ({}): {} tokens", 
            contract.name, contract.ticker, contract.balance);
        log::debug!("      Contract: {}", contract.contract_id);
    }
    
    // Show UTXOs with RGB assets
    let occupied_utxos: Vec<_> = balance.utxos.iter().filter(|u| u.is_occupied).collect();
    if !occupied_utxos.is_empty() {
        log::debug!("      RGB UTXOs: {}", occupied_utxos.len());
        for utxo in occupied_utxos {
            log::debug!("        {}:{} ({} sats, {} assets)", 
                utxo.txid, utxo.vout, utxo.amount_sats, utxo.bound_assets.len());
        }
    }
}

/// Find the correct vout for a transaction by querying Esplora
/// Returns the vout that matches the expected amount
pub async fn find_vout_for_amount(
    txid: &str,
    expected_amount_sats: u64,
    esplora_url: &str,
) -> anyhow::Result<u32> {
    let url = format!("{}/tx/{}", esplora_url, txid);
    
    log::debug!("Querying transaction to find vout with {} sats", expected_amount_sats);
    
    let resp = reqwest::get(&url).await?;
    let tx = resp.json::<serde_json::Value>().await?;
    
    // Search through outputs to find matching amount
    if let Some(vouts) = tx["vout"].as_array() {
        for (index, vout) in vouts.iter().enumerate() {
            if let Some(value) = vout["value"].as_u64() {
                log::debug!("  vout {}: {} sats", index, value);
                if value == expected_amount_sats {
                    log::info!("✓ Found matching vout: {} (amount: {} sats)", index, value);
                    return Ok(index as u32);
                }
            }
        }
    }
    
    anyhow::bail!("Could not find vout with amount {} sats in transaction {}", expected_amount_sats, txid)
}

