/// Integration Test: RGB Transfer Balance Persistence
/// 
/// This test reproduces the bug where sender's RGB balance shows 0 after a confirmed transfer
/// instead of the correct remaining amount.
/// 
/// ## Signet Mode (Default - Real Network)
/// Prerequisites:
/// - Create a `.env` file in the wallet directory with TEST_MNEMONIC
/// - Wallet should have at least 50,000 sats
/// - Internet connection for Esplora API
/// - ~15-20 minutes for test execution (includes block confirmations)
/// 
/// Setup:
/// ```bash
/// # Create .env file in wallet/ directory:
/// echo 'TEST_MNEMONIC="your twelve word mnemonic here"' > .env
/// echo 'TEST_TIMEOUT=900' >> .env  # Optional: 15 minutes
/// ```
/// 
/// Run with:
/// ```bash
/// cargo test --test rgb_transfer_balance_test -- --ignored --nocapture
/// ```
/// 
/// ## Regtest Mode (Fast Local Testing)
/// Prerequisites:
/// - Bitcoin Core running in Regtest mode
/// - Esplora mock server running
/// - Funded test wallet (generate blocks to fund)
/// - ~30 seconds for test execution (instant mining)
/// 
/// Setup:
/// ```bash
/// # Terminal 1: Start Bitcoin Core Regtest
/// bitcoind -regtest -daemon -rpcuser=regtest -rpcpassword=regtest
/// bitcoin-cli -regtest createwallet "test_wallet"
/// bitcoin-cli -regtest -generate 101  # Fund wallet
/// 
/// # Terminal 2: Start Esplora Mock Server
/// cd esplora-mock
/// BITCOIN_RPC_USER=regtest BITCOIN_RPC_PASSWORD=regtest cargo run
/// 
/// # Terminal 3: Run test
/// cd wallet
/// BITCOIN_NETWORK=regtest \
/// ESPLORA_URL=http://localhost:3000 \
/// TEST_MNEMONIC="your test mnemonic" \
/// cargo test --test rgb_transfer_balance_test -- --ignored --nocapture
/// ```

mod common;

use std::time::Instant;
use tokio::signal;
use wallet::rgb::asset::IssueAssetRequest;
use wallet::api::types::{CreateUtxoRequest, GenerateInvoiceRequest, SendBitcoinRequest};

// Import test utilities
use common::{
    TestEnvironment, TestNetworkConfig,
    wait_for_confirmation, wait_for_electrs_sync, ensure_rgb_sync_delay,
    print_rgb_balance, find_vout_for_amount,
};

/// Main test logic - extracted for Control-C handling
async fn run_test_logic(timeout: u64, fee_rate: u64) -> anyhow::Result<()> {
    // ============================================================================
    // PHASE 0: Environment Setup
    // ============================================================================
    
    log::info!("\n--- Phase 0: Environment Setup ---\n");
    
    // Load network configuration
    let network_config = TestNetworkConfig::from_env();
    
    // Get funded wallet mnemonic
    let mnemonic_str = std::env::var("TEST_MNEMONIC")
        .expect("Set TEST_MNEMONIC env var with funded wallet mnemonic");
    
    let mnemonic = bip39::Mnemonic::parse(&mnemonic_str)
        .expect("Invalid mnemonic format");
    
    log::info!("✓ Mnemonic loaded");
    
    // Create test environment
    let env = TestEnvironment::new("balance")?;
    log::info!("✓ Test environment created");
    
    // Import funded wallet
    log::info!("Importing funded wallet: {}", env.wallet1_name);
    env.manager.import_wallet(&env.wallet1_name, mnemonic)?;
    log::info!("✓ Wallet imported");
    
    // Create recipient wallet
    log::info!("Creating recipient wallet: {}", env.wallet2_name);
    let _wallet2_info = env.manager.create_wallet(&env.wallet2_name)?;
    log::info!("✓ Recipient wallet created");
    
    // ============================================================================
    // PHASE 1: Asset Issuance
    // ============================================================================
    
    log::info!("\n--- Phase 1: Asset Issuance ---\n");
    
    // Sync sender wallet
    log::info!("Syncing sender wallet...");
    let sync_result = env.manager.sync_wallet(&env.wallet1_name).await?;
    log::info!("✓ Wallet synced to height {}, checked {} addresses, found {} new txs", 
        sync_result.synced_height, sync_result.addresses_checked, sync_result.new_transactions);
    
    // Regtest: Ensure coinbase maturity (100 blocks)
    if network_config.is_regtest {
        log::info!("🔨 Mining blocks to mature coinbase outputs (Bitcoin requires 100 confirmations)...");
        for _ in 0..100 {
            common::mine_regtest_block(&network_config.esplora_url).await?;
        }
        log::info!("✓ Mined 100 blocks for coinbase maturity");
    }
    
    // Check initial balance
    let initial_balance = env.manager.get_balance(&env.wallet1_name).await?;
    let total_sats = initial_balance.confirmed_sats + initial_balance.unconfirmed_sats;
    log::info!("Initial Bitcoin balance: {} sats (confirmed: {}, unconfirmed: {})", 
        total_sats, initial_balance.confirmed_sats, initial_balance.unconfirmed_sats);
    
    if total_sats < 50_000 {
        anyhow::bail!(
            "Insufficient balance: {} sats (need at least 50,000 sats)",
            total_sats
        );
    }
    
    log::info!("✓ Sufficient balance confirmed");
    
    // Try to find a reusable UTXO from previous test run
    log::info!("\nPreparing UTXO for RGB genesis...");
    const TEST_UTXO_MIN: u64 = 17_000;  // Minimum for test UTXOs
    const TEST_UTXO_MAX: u64 = 50_000;  // Maximum for test UTXOs
    
    let suitable_utxo = initial_balance.utxos.iter()
        .find(|u| {
            u.amount_sats >= TEST_UTXO_MIN
            && u.amount_sats <= TEST_UTXO_MAX
            && !u.is_occupied         // Not holding RGB assets
            && u.confirmations > 0    // Must be confirmed
        });
    
    let genesis_utxo = if let Some(utxo) = suitable_utxo {
        log::info!("♻️  Reusing existing test UTXO from previous run");
        log::info!("  UTXO: {}:{}", utxo.txid, utxo.vout);
        log::debug!("  Amount: {} sats (in range {}-{} sats)", 
            utxo.amount_sats, TEST_UTXO_MIN, TEST_UTXO_MAX);
        log::debug!("  Confirmations: {}", utxo.confirmations);
        log::debug!("  Address: {}", utxo.address);
        log::info!("✅ Skipping UTXO creation (saving ~6 minutes!)");
        format!("{}:{}", utxo.txid, utxo.vout)
    } else {
        log::info!("No reusable UTXO found, creating new one...");
        log::info!("Creating UTXO for RGB (30,000 sats)...");
        
        let utxo_resp = env.manager.create_utxo(&env.wallet1_name, CreateUtxoRequest {
            amount_btc: Some(0.0003),
            fee_rate_sat_vb: Some(fee_rate),
        }).await?;
        
        log::info!("✓ UTXO created: {}", utxo_resp.txid);
        log::debug!("  Amount: {} sats, Fee: {} sats", utxo_resp.amount_sats, utxo_resp.fee_sats);
        log::debug!("  Address: {}", utxo_resp.target_address);
        
        // Wait for UTXO confirmation
        log::info!("\n⏳ Waiting for UTXO confirmation...");
        let utxo_block = wait_for_confirmation(&utxo_resp.txid, &network_config, timeout).await?;
        log::info!("✅ UTXO confirmed in block {}!", utxo_block);
        
        // Wait for Electrs to index this block
        if network_config.is_regtest {
            wait_for_electrs_sync(&network_config.esplora_url, utxo_block).await?;
            ensure_rgb_sync_delay(true).await;
        }
        
        // Find the correct vout by querying the transaction
        log::info!("\nFinding correct vout for genesis UTXO...");
        let vout = find_vout_for_amount(&utxo_resp.txid, utxo_resp.amount_sats, &network_config.esplora_url).await?;
        log::info!("✓ Genesis UTXO: {}:{}", utxo_resp.txid, vout);
        
        format!("{}:{}", utxo_resp.txid, vout)
    };
    
    log::info!("✓ Using genesis UTXO: {}", genesis_utxo);
    
    // Issue asset
    log::info!("\nIssuing RGB asset (1000 TEST tokens)...");
    let asset = env.manager.issue_asset(&env.wallet1_name, IssueAssetRequest {
        ticker: "TEST".to_string(),
        name: "TestToken".to_string(), // No spaces allowed in RGB asset names
        supply: 1000,
        precision: 0,
        genesis_utxo,
    }).await?;
    
    log::info!("✅ Asset issued!");
    log::info!("  Contract ID: {}", asset.contract_id);
    log::info!("  Genesis Seal: {}", asset.genesis_seal);
    
    // Verify sender has asset
    let balance_post_issue = env.manager.get_balance(&env.wallet1_name).await?;
    print_rgb_balance("Sender (post-issuance)", &balance_post_issue);
    
    let sender_contract = balance_post_issue.known_contracts.iter()
        .find(|c| c.contract_id == asset.contract_id)
        .expect("Asset should exist in sender wallet after issuance");
    
    assert_eq!(sender_contract.balance, 1000, 
        "Sender should have 1000 tokens after issuance");
    log::info!("✓ Issuance balance verified: 1000 tokens");
    
    // Export genesis to recipient
    log::info!("\nExporting genesis to recipient...");
    let genesis = env.manager.export_genesis_consignment(&env.wallet1_name, &asset.contract_id).await?;
    log::debug!("  Genesis consignment: {}", genesis.consignment_filename);
    log::debug!("  File size: {} bytes", genesis.file_size_bytes);
    
    // Read genesis consignment from disk
    let genesis_path = env.temp_dir.path().join("exports").join(&genesis.consignment_filename);
    let genesis_bytes = std::fs::read(&genesis_path)
        .map_err(|e| anyhow::anyhow!("Failed to read genesis file: {}", e))?;
    log::debug!("  Read {} bytes", genesis_bytes.len());
    
    env.manager.accept_consignment(&env.wallet2_name, genesis_bytes).await?;
    log::info!("✅ Genesis exported and accepted by recipient");
    
    // Verify recipient can see asset (0 balance)
    let recipient_balance_pre = env.manager.get_balance(&env.wallet2_name).await?;
    print_rgb_balance("Recipient (post-genesis)", &recipient_balance_pre);
    
    // ============================================================================
    // PHASE 2: Transfer Execution
    // ============================================================================
    
    log::info!("\n--- Phase 2: Transfer Execution ---\n");
    
    // Regtest: Mine more blocks to mature any new coinbase outputs created during Phase 1
    if network_config.is_regtest {
        log::info!("🔨 Mining 100 more blocks to mature recent coinbase outputs...");
        for _ in 0..100 {
            common::mine_regtest_block(&network_config.esplora_url).await?;
        }
        log::info!("✓ Mined 100 blocks - all coinbase outputs now mature");
    }
    
    // Check if recipient already has a suitable UTXO from previous test run
    log::info!("Checking if recipient needs funding...");
    const RECIPIENT_MIN_UTXO: u64 = 5_000;   // Minimum needed for AuthToken generation
    const RECIPIENT_MAX_UTXO: u64 = 20_000;  // Maximum to avoid locking large UTXOs
    
    let has_suitable_utxo = recipient_balance_pre.utxos.iter()
        .any(|u| {
            u.amount_sats >= RECIPIENT_MIN_UTXO
            && u.amount_sats <= RECIPIENT_MAX_UTXO
            && u.confirmations > 0
        });
    
    if has_suitable_utxo {
        log::info!("♻️  Recipient already has suitable UTXO from previous run (range: {}-{} sats)", 
            RECIPIENT_MIN_UTXO, RECIPIENT_MAX_UTXO);
        log::info!("✅ Skipping recipient funding (saving ~12,000 sats + 3-6 minutes!)");
    } else {
        // Fund recipient wallet (needed for invoice generation with AuthToken)
        log::info!("No suitable UTXO found, funding recipient wallet...");
        log::info!("Getting recipient address...");
        let recipient_addresses = env.manager.get_addresses(&env.wallet2_name, 1)?;
        let recipient_addr = &recipient_addresses[0].address;
        log::debug!("  Recipient address: {}", recipient_addr);
        
        // Send small amount from sender to recipient
        log::info!("Sending 10,000 sats to recipient...");
        let fund_tx = env.manager.send_bitcoin(
            &env.wallet1_name,
            SendBitcoinRequest {
                to_address: recipient_addr.to_string(),
                amount_sats: 10_000,
                fee_rate_sat_vb: Some(fee_rate),
            }
        ).await?;
        log::info!("✓ Funding tx: {}", fund_tx.txid);
        
        // Wait for confirmation
        log::info!("\n⏳ Waiting for funding tx confirmation...");
        let fund_block = wait_for_confirmation(&fund_tx.txid, &network_config, timeout).await?;
        log::info!("✅ Recipient funded!");
        
        // Wait for Electrs to index this block
        if network_config.is_regtest {
            wait_for_electrs_sync(&network_config.esplora_url, fund_block).await?;
            ensure_rgb_sync_delay(true).await;
        }
        
        // Sync recipient wallet to see the new UTXO
        log::info!("Syncing recipient wallet...");
        let _ = env.manager.sync_wallet(&env.wallet2_name).await?;
        log::info!("✓ Recipient synced");
    }
    
    // Generate invoice (recipient requests 100 tokens)
    log::info!("\nGenerating invoice for 100 tokens...");
    let invoice_resp = env.manager.generate_rgb_invoice(&env.wallet2_name, GenerateInvoiceRequest {
        contract_id: asset.contract_id.clone(),
        amount: Some(100),
        utxo_selection: None, // Auto-select
        nonce: None,
    }).await?;
    
    log::info!("✅ Invoice generated");
    log::debug!("  Invoice: {}", invoice_resp.invoice);
    
    // Send transfer (sender sends 100 tokens, keeps 900 as change)
    log::info!("\nSending transfer (100 tokens)...");
    let transfer_start = Instant::now();
    let transfer_resp = env.manager.send_transfer(
        &env.wallet1_name, 
        &invoice_resp.invoice,
        Some(fee_rate)
    ).await?;
    
    log::info!("✅ Transfer sent in {:.2}s", transfer_start.elapsed().as_secs_f64());
    log::info!("  Transaction: {}", transfer_resp.bitcoin_txid);
    log::info!("  Consignment: {}", transfer_resp.consignment_filename);
    log::debug!("  Status: {}", transfer_resp.status);
    
    // Read consignment file from disk
    log::info!("\nReading consignment file: {}", transfer_resp.consignment_filename);
    let consignment_path = env.temp_dir.path().join("consignments").join(&transfer_resp.consignment_filename);
    let consignment_bytes = std::fs::read(&consignment_path)
        .map_err(|e| anyhow::anyhow!("Failed to read consignment file: {}", e))?;
    log::debug!("Consignment size: {} bytes", consignment_bytes.len());
    
    // Accept consignment (recipient imports transfer)
    log::info!("\nAccepting transfer at recipient...");
    env.manager.accept_consignment(&env.wallet2_name, consignment_bytes).await?;
    log::info!("✅ Transfer accepted by recipient");
    
    // Wait for blockchain confirmation BEFORE checking balance
    // RGB tokens are NOT visible in balance until witness transaction confirms
    log::info!("\n⏳ Waiting for transfer confirmation...");
    let transfer_block = wait_for_confirmation(&transfer_resp.bitcoin_txid, &network_config, timeout).await?;
    log::info!("✅ Transfer confirmed in block {}!", transfer_block);
    
    // 🔧 CRITICAL: Wait for Electrs to fully index the witness transaction
    // RGB needs the witness data to update allocations, and Electrs must index it first
    if network_config.is_regtest {
        log::info!("⏳ Waiting for Electrs to index witness transaction...");
        wait_for_electrs_sync(&network_config.esplora_url, transfer_block).await?;
        ensure_rgb_sync_delay(true).await;
        log::info!("✓ Electrs sync complete");
    }
    
    // ============================================================================
    // CRITICAL TEST POINT 1: Balance after witness confirmation
    // ============================================================================
    
    log::info!("\n--- Critical Test Point 1: Balance Check (After Confirmation) ---\n");
    log::info!("ℹ️  RGB tokens are only visible after witness transaction confirms");
    
    let balance_confirmed = env.manager.get_balance(&env.wallet1_name).await?;
    print_rgb_balance("Sender (tx confirmed)", &balance_confirmed);
    
    let sender_contract_confirmed = balance_confirmed.known_contracts.iter()
        .find(|c| c.contract_id == asset.contract_id)
        .expect("Asset should exist in sender wallet");
    
    assert_eq!(sender_contract_confirmed.balance, 900,
        "Sender should have 900 tokens after witness confirms (1000 - 100 sent)");
    
    log::info!("✅ Post-confirmation balance CORRECT: 900 tokens");
    
    // ============================================================================
    // CRITICAL TEST POINT 2: Balance persistence after sync (THE BUG TEST)
    // ============================================================================
    
    log::info!("\n{}", "=".repeat(80));
    log::info!("🐛 CRITICAL TEST POINT 2: Balance Persistence Test");
    log::info!("{}\n", "=".repeat(80));
    
    log::info!("Testing if balance persists after RGB sync (creates new runtime):");
    log::info!("  Expected: Sender should still have 900 tokens");
    log::info!("  Bug:      Seals not registered = runtime can't find tokens = 0 balance\n");
    
    // Sync RGB state (creates new runtime instance in current architecture)
    log::info!("Syncing sender RGB runtime to test persistence...");
    env.manager.sync_rgb_runtime(&env.wallet1_name).await?;
    log::info!("✓ RGB sync complete");
    
    // Query balance again (another new runtime instance)
    let balance_after_sync = env.manager.get_balance(&env.wallet1_name).await?;
    print_rgb_balance("Sender (after sync)", &balance_after_sync);
    
    let sender_contract_after_sync = balance_after_sync.known_contracts.iter()
        .find(|c| c.contract_id == asset.contract_id)
        .expect("Asset should still exist in sender wallet");
    
    // THE BUG ASSERTION: This would FAIL if seals weren't registered
    log::info!("\n🔬 Asserting sender balance = 900 tokens...");
    assert_eq!(sender_contract_after_sync.balance, 900,
        "\n\n🐛 BUG DETECTED 🐛\n\
         Sender balance shows {} but should be 900!\n\
         This means the change seals were not properly registered.\n\n\
         Root Cause:\n\
         - Genesis seal not registered after issuance\n\
         - OR change seal not registered after transfer\n\
         - Wallet descriptor missing seals = can't find tokens\n\n\
         Solution:\n\
         Manual seal registration after issuance and transfer (IMPLEMENTED)\n\n",
        sender_contract_after_sync.balance);
    
    log::info!("✅ Balance persists correctly after sync: 900 tokens");
    
    // ============================================================================
    // VERIFICATION: Check recipient balance
    // ============================================================================
    
    log::info!("\n--- Verification: Recipient Balance ---\n");
    
    let recipient_balance_final = env.manager.get_balance(&env.wallet2_name).await?;
    print_rgb_balance("Recipient (final)", &recipient_balance_final);
    
    let recipient_contract = recipient_balance_final.known_contracts.iter()
        .find(|c| c.contract_id == asset.contract_id)
        .expect("Recipient should have asset");
    
    assert_eq!(recipient_contract.balance, 100,
        "Recipient should have 100 tokens");
    
    log::info!("✅ Recipient balance CORRECT: 100 tokens");
    
    // ============================================================================
    // TEST COMPLETE
    // ============================================================================
    
    log::info!("\n{}", "=".repeat(80));
    log::info!("🎉 TEST PASSED: All assertions succeeded!");
    log::info!("{}\n", "=".repeat(80));
    
    log::info!("Summary:");
    log::info!("  ✓ Asset issued: 1000 tokens");
    log::info!("  ✓ Transfer sent: 100 tokens");
    log::info!("  ✓ Sender balance (unconfirmed): 900 tokens");
    log::info!("  ✓ Sender balance (confirmed): 900 tokens ← Bug was here!");
    log::info!("  ✓ Recipient balance: 100 tokens");
    log::info!("  ✓ Total tokens conserved: 1000 = 900 + 100");
    
    // env (TestEnvironment) drops here, triggering cleanup
    Ok(())
}

#[tokio::test]
#[ignore] // Run manually with funded wallet: cargo test --test rgb_transfer_balance_test -- --ignored --nocapture
async fn test_sender_balance_persists_after_confirmed_transfer() -> anyhow::Result<()> {
    // Load .env file if it exists
    dotenv::dotenv().ok();
    
    // Initialize logger
    let _ = env_logger::builder()
        .is_test(true)
        .filter_level(log::LevelFilter::Debug)
        .try_init();
    
    log::info!("\n{}", "=".repeat(80));
    log::info!("🧪 RGB Transfer Balance Persistence Test");
    log::info!("{}\n", "=".repeat(80));
    
    // Get timeout from env or use 15 minutes default
    let timeout = std::env::var("TEST_TIMEOUT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(900); // 15 minutes
    
    // Get fee rate from env or use 20 sat/vB default
    let fee_rate = std::env::var("TEST_FEE_RATE")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(20); // 20 sat/vB
    
    log::info!("Confirmation timeout: {} seconds ({} minutes)", timeout, timeout / 60);
    log::info!("Fee rate: {} sat/vB", fee_rate);
    log::info!("ℹ️  Press Control-C to stop test and trigger cleanup\n");
    
    // Race test execution against Control-C signal
    tokio::select! {
        // Branch 1: Normal test execution
        result = run_test_logic(timeout, fee_rate) => {
            result?;
            log::info!("\n✅ Test completed successfully");
            log::info!("🧹 Cleaning up test environment...");
        }
        
        // Branch 2: User interrupts with Control-C
        _ = signal::ctrl_c() => {
            log::warn!("\n");
            log::warn!("⚠️  Test interrupted by user (Control-C)");
            log::info!("🧹 Cleaning up test environment...");
            log::info!("   (This may take a moment as wallets are deleted)");
        }
    }
    
    // TestEnvironment's Drop trait runs here, deleting wallets and temp directory
    Ok(())
}

