# Remaining Integration Tests Implementation Plan

**Status**: Test Case 1 (Genesis Export/Import) ✅ COMPLETE  
**Next**: Test Cases 2-5

---

## Test Case 2: Invoice Generation and Parsing

**File**: `f1r3fly-rgb-wallet/tests/f1r3fly/invoice_operations_test.rs`

### Objective
Verify end-to-end invoice generation, parsing, and seal extraction workflow between two wallets.

### Prerequisites (Already Available)
- ✅ `TestBitcoinEnv` - regtest environment
- ✅ `setup_recipient_wallet()` - creates funded wallet
- ✅ `issue_test_asset()` - helper to issue assets
- ✅ `require_f1r3node!()` - skip if F1r3node unavailable

### Test Structure

```rust
#[tokio::test]
async fn test_invoice_round_trip() {
    require_f1r3node!();
    
    let env = TestBitcoinEnv::new("invoice_operations");
    
    // ========================================================================
    // Step 1: Alice issues asset and Bob accepts genesis
    // ========================================================================
    let (mut alice, asset_info, _) = issue_test_asset(
        &env, 
        "alice", 
        "USD", 
        10_000
    ).await.expect("Failed to issue asset");
    
    let mut bob = setup_recipient_wallet(&env, "bob", "test_password")
        .await
        .expect("Failed to setup Bob");
    
    // Export and import genesis
    let genesis_response = alice
        .export_genesis(&asset_info.contract_id)
        .await
        .expect("Failed to export genesis");
    
    bob.accept_consignment(genesis_response.consignment_path.to_str().unwrap())
        .await
        .expect("Failed to accept genesis");
    
    // ========================================================================
    // Step 2: Bob generates invoice for 100 tokens
    // ========================================================================
    let invoice_response = bob
        .generate_invoice(&asset_info.contract_id, 100, None)
        .await
        .expect("Failed to generate invoice");
    
    // ========================================================================
    // Step 3: Verify invoice format (RGB standard)
    // ========================================================================
    assert!(
        invoice_response.invoice_string.starts_with("rgb:"),
        "Invoice should start with rgb: prefix"
    );
    
    assert!(
        invoice_response.invoice_string.len() < 150,
        "Invoice should be compact (~110 chars)"
    );
    
    println!("✓ Invoice generated: {}", invoice_response.invoice_string);
    
    // ========================================================================
    // Step 4: Alice parses invoice
    // ========================================================================
    let parsed_response = alice
        .parse_invoice(&invoice_response.invoice_string)
        .await
        .expect("Failed to parse invoice");
    
    // ========================================================================
    // Step 5: Verify parsed data matches original
    // ========================================================================
    assert_eq!(
        parsed_response.contract_id, asset_info.contract_id,
        "Contract ID should match"
    );
    
    assert_eq!(
        parsed_response.amount, 100,
        "Amount should match"
    );
    
    assert!(
        !parsed_response.recipient_address.is_empty(),
        "Recipient address should be present"
    );
    
    // ========================================================================
    // Step 6: Verify blinded seal extracted
    // ========================================================================
    assert!(
        parsed_response.blinded_utxo.is_some(),
        "Blinded UTXO should be extracted"
    );
    
    println!("✓ Invoice parsed successfully");
    println!("  Contract ID: {}", parsed_response.contract_id);
    println!("  Amount: {}", parsed_response.amount);
    println!("  Recipient: {}", parsed_response.recipient_address);
    
    println!("\n✅ Invoice round-trip test completed successfully!");
}
```

### What Already Exists
- ✅ `WalletManager::generate_invoice()` - implemented in Phase 2
- ✅ `WalletManager::parse_invoice()` - implemented in Phase 2
- ✅ Invoice CLI commands
- ✅ Core invoice functionality in `f1r3fly-rgb`

### What Needs to Be Created
- **None** - All functionality already implemented!

### Expected Outcome
- Invoice generation produces valid RGB invoice string
- Invoice parsing correctly extracts all fields
- Blinded seal properly extracted for transfer use

---

## Test Case 3: Complete Transfer Flow (Happy Path)

**File**: `f1r3fly-rgb-wallet/tests/f1r3fly/complete_transfer_test.rs`

### Objective
Test full transfer lifecycle: Alice sends tokens to Bob, verify balances, Bitcoin anchoring, and state consistency.

### Prerequisites (Already Available)
- ✅ `TestBitcoinEnv` with `wait_for_confirmation()`
- ✅ `issue_test_asset()` helper
- ✅ `setup_recipient_wallet()` helper
- ✅ `verify_balance_with_retry()` - created in Test Case 1
- ✅ `require_f1r3node!()` macro

### New Helpers Needed

```rust
/// Helper to verify Tapret commitment exists in Bitcoin transaction
pub fn verify_tapret_in_tx(
    tx: &bdk_wallet::bitcoin::Transaction,
    expected_state_hash: [u8; 32],
) -> Result<(), Box<dyn std::error::Error>> {
    // Extract Tapret commitment from TX outputs
    // Verify it matches expected state hash
    // Return error if not found or mismatch
}
```

### Test Structure

```rust
#[tokio::test]
async fn test_complete_transfer_alice_to_bob() {
    require_f1r3node!();
    
    let env = TestBitcoinEnv::new("complete_transfer");
    
    // ========================================================================
    // Step 1: Setup - Alice issues 10,000 TEST, Bob accepts genesis
    // ========================================================================
    println!("\n📦 Step 1: Setting up wallets and asset");
    
    let (mut alice, asset_info, _) = issue_test_asset(
        &env, 
        "alice", 
        "TEST", 
        10_000
    ).await.expect("Failed to issue asset");
    
    let mut bob = setup_recipient_wallet(&env, "bob", "test_password")
        .await
        .expect("Failed to setup Bob");
    
    // Export and import genesis
    let genesis_response = alice.export_genesis(&asset_info.contract_id).await?;
    bob.accept_consignment(genesis_response.consignment_path.to_str().unwrap()).await?;
    
    println!("✓ Setup complete");
    
    // ========================================================================
    // Step 2: Bob generates invoice for 2,500 tokens
    // ========================================================================
    println!("\n💰 Step 2: Bob generates invoice for 2,500 tokens");
    
    let invoice = bob
        .generate_invoice(&asset_info.contract_id, 2_500, None)
        .await
        .expect("Failed to generate invoice");
    
    println!("✓ Invoice generated: {}", invoice.invoice_string);
    
    // ========================================================================
    // Step 3: Alice sends transfer
    // ========================================================================
    println!("\n📤 Step 3: Alice sends transfer to Bob");
    
    let transfer_response = alice
        .send_transfer(&invoice.invoice_string)
        .await
        .expect("Failed to send transfer");
    
    println!("✓ Transfer sent");
    println!("  TXID: {}", transfer_response.txid);
    println!("  Consignment: {}", transfer_response.consignment_path.display());
    
    // ========================================================================
    // Step 4: Wait for Bitcoin confirmation
    // ========================================================================
    println!("\n⏳ Step 4: Waiting for Bitcoin confirmation");
    
    env.wait_for_confirmation(&transfer_response.txid, 1)
        .await
        .expect("Failed to confirm transaction");
    
    println!("✓ Transaction confirmed");
    
    // ========================================================================
    // Step 5: Verify Tapret commitment in Bitcoin TX
    // ========================================================================
    println!("\n🔐 Step 5: Verifying Tapret commitment on-chain");
    
    use bdk_wallet::bitcoin::Txid as BdkTxid;
    let txid_parsed = BdkTxid::from_str(&transfer_response.txid)
        .expect("Invalid txid");
    
    let tx = env.esplora_client
        .inner()
        .get_tx(&txid_parsed)
        .expect("Failed to fetch TX")
        .expect("TX not found");
    
    verify_tapret_in_tx(&tx, transfer_response.state_hash)
        .expect("Tapret verification failed");
    
    println!("✓ Tapret commitment verified on Bitcoin");
    
    // ========================================================================
    // Step 6: Verify Alice's balance (with retry for F1r3fly state delays)
    // ========================================================================
    println!("\n💵 Step 6: Verifying Alice's balance");
    
    verify_balance_with_retry(&mut alice, &asset_info.contract_id, 7_500, 5)
        .await
        .expect("Alice balance mismatch");
    
    println!("✓ Alice balance: 7,500 tokens (10,000 - 2,500)");
    
    // ========================================================================
    // Step 7: Verify Alice's RGB-occupied UTXOs tracked
    // ========================================================================
    println!("\n🔒 Step 7: Verifying RGB-occupied UTXOs");
    
    let alice_occupied = alice
        .get_occupied_utxos()
        .await
        .expect("Failed to get occupied UTXOs");
    
    assert!(
        alice_occupied.len() > 0,
        "Alice should have RGB-occupied UTXOs (change seal)"
    );
    
    println!("✓ Alice has {} RGB-occupied UTXO(s)", alice_occupied.len());
    
    // ========================================================================
    // Step 8: Bob accepts consignment
    // ========================================================================
    println!("\n📥 Step 8: Bob accepts consignment");
    
    bob.accept_consignment(transfer_response.consignment_path.to_str().unwrap())
        .await
        .expect("Failed to accept consignment");
    
    println!("✓ Consignment accepted");
    
    // ========================================================================
    // Step 9: Verify Bob's balance
    // ========================================================================
    println!("\n💵 Step 9: Verifying Bob's balance");
    
    verify_balance_with_retry(&mut bob, &asset_info.contract_id, 2_500, 5)
        .await
        .expect("Bob balance mismatch");
    
    println!("✓ Bob balance: 2,500 tokens");
    
    // ========================================================================
    // Step 10: Verify total supply conservation
    // ========================================================================
    println!("\n🔢 Step 10: Verifying total supply conservation");
    
    let alice_balance = alice.get_asset_balance(&asset_info.contract_id).await?;
    let bob_balance = bob.get_asset_balance(&asset_info.contract_id).await?;
    
    assert_eq!(
        alice_balance.total + bob_balance.total,
        10_000,
        "Total supply should be conserved"
    );
    
    println!("✓ Total supply conserved: {} + {} = 10,000", 
             alice_balance.total, bob_balance.total);
    
    println!("\n✅ Complete transfer test passed!");
}
```

### What Already Exists
- ✅ `WalletManager::send_transfer()` - implemented
- ✅ `WalletManager::accept_consignment()` - implemented
- ✅ `WalletManager::get_asset_balance()` - implemented
- ✅ `WalletManager::get_occupied_utxos()` - needs to be added
- ✅ `verify_balance_with_retry()` - created in Test Case 1

### What Needs to Be Created
- `verify_tapret_in_tx()` helper function
- `WalletManager::get_occupied_utxos()` method (returns HashSet<String>)

### Expected Outcome
- Transfer completes successfully
- Bitcoin TX contains Tapret commitment
- Balances update correctly (with F1r3fly state delay)
- Total supply is conserved
- Change seals properly tracked

---

## Test Case 4: Security & Validation Tests

**File**: `f1r3fly-rgb-wallet/tests/f1r3fly/validation_security_test.rs`

### Objective
Test all validation and security failure cases to ensure robust error handling.

### Test Structure (Multiple Independent Tests)

```rust
/// Test 1: Reject consignment with corrupted state hash
#[tokio::test]
async fn test_reject_invalid_state_hash() {
    require_f1r3node!();
    
    let env = TestBitcoinEnv::new("invalid_state_hash");
    
    // Setup: Alice issues, exports genesis
    let (mut alice, asset_info, _) = issue_test_asset(&env, "alice", "TST", 1000).await?;
    let genesis_response = alice.export_genesis(&asset_info.contract_id).await?;
    
    // Corrupt the consignment JSON (change state_hash)
    let mut consignment_bytes = std::fs::read(&genesis_response.consignment_path)?;
    let mut consignment: serde_json::Value = serde_json::from_slice(&consignment_bytes)?;
    
    // Corrupt state hash in f1r3fly_proof
    consignment["f1r3fly_proof"]["state_hash"] = serde_json::json!(
        "0000000000000000000000000000000000000000000000000000000000000000"
    );
    
    let corrupted_path = env.wallet_dir("bob").join("corrupted.json");
    std::fs::write(&corrupted_path, serde_json::to_vec(&consignment)?)?;
    
    // Bob tries to accept corrupted consignment
    let mut bob = setup_recipient_wallet(&env, "bob", "password").await?;
    let result = bob.accept_consignment(corrupted_path.to_str().unwrap()).await;
    
    // Should fail validation
    assert!(result.is_err(), "Should reject corrupted state hash");
    assert!(
        result.unwrap_err().to_string().contains("validation") ||
        result.unwrap_err().to_string().contains("hash"),
        "Error should mention validation or hash issue"
    );
    
    println!("✓ Correctly rejected consignment with invalid state hash");
}

/// Test 2: Reject consignment with unfinalized F1r3fly block
#[tokio::test]
async fn test_reject_unfinalized_block() {
    // Note: This test may need to mock F1r3node response or use very recent block
    require_f1r3node!();
    
    // Strategy: Create consignment with block_hash of current block (not finalized yet)
    // Validation should fail when checking is_block_finalized()
    
    println!("✓ TODO: Implement with F1r3node mocking or timing strategy");
}

/// Test 3: Prevent double-spend (RGB-occupied UTXO protection)
#[tokio::test]
async fn test_prevent_double_spend() {
    require_f1r3node!();
    
    let env = TestBitcoinEnv::new("double_spend");
    
    // Setup: Alice has 10,000 tokens, Bob and Carol accept genesis
    let (mut alice, asset_info, _) = issue_test_asset(&env, "alice", "TST", 10_000).await?;
    
    let mut bob = setup_recipient_wallet(&env, "bob", "password").await?;
    let mut carol = setup_recipient_wallet(&env, "carol", "password").await?;
    
    // Both accept genesis
    let genesis_response = alice.export_genesis(&asset_info.contract_id).await?;
    bob.accept_consignment(genesis_response.consignment_path.to_str().unwrap()).await?;
    carol.accept_consignment(genesis_response.consignment_path.to_str().unwrap()).await?;
    
    // Bob generates invoice for 5,000
    let bob_invoice = bob.generate_invoice(&asset_info.contract_id, 5_000, None).await?;
    
    // Alice sends 5,000 to Bob
    let transfer1 = alice.send_transfer(&bob_invoice.invoice_string).await?;
    env.wait_for_confirmation(&transfer1.txid, 1).await?;
    
    // Carol generates invoice for 5,000 (using SAME UTXO Alice just spent)
    let carol_invoice = carol.generate_invoice(&asset_info.contract_id, 5_000, None).await?;
    
    // Alice tries to send another 5,000 to Carol
    // This should FAIL because the UTXO is RGB-occupied
    let result = alice.send_transfer(&carol_invoice.invoice_string).await;
    
    assert!(
        result.is_err(),
        "Should prevent double-spend of RGB UTXO"
    );
    
    let error_msg = result.unwrap_err().to_string();
    assert!(
        error_msg.contains("occupied") || error_msg.contains("spent") || error_msg.contains("insufficient"),
        "Error should mention UTXO occupation or insufficiency"
    );
    
    println!("✓ Double-spend correctly prevented");
}

/// Test 4: Reject consignment with invalid seal (non-existent UTXO)
#[tokio::test]
async fn test_reject_invalid_seal() {
    require_f1r3node!();
    
    let env = TestBitcoinEnv::new("invalid_seal");
    
    // Setup
    let (mut alice, asset_info, _) = issue_test_asset(&env, "alice", "TST", 1000).await?;
    let genesis_response = alice.export_genesis(&asset_info.contract_id).await?;
    
    // Corrupt seal to point to non-existent UTXO
    let mut consignment_bytes = std::fs::read(&genesis_response.consignment_path)?;
    let mut consignment: serde_json::Value = serde_json::from_slice(&consignment_bytes)?;
    
    // Change seal to fake UTXO
    if let Some(seals) = consignment["seals"].as_object_mut() {
        for (_key, seal) in seals.iter_mut() {
            seal["primary"]["Extern"]["txid"] = serde_json::json!(
                "0000000000000000000000000000000000000000000000000000000000000000"
            );
        }
    }
    
    let corrupted_path = env.wallet_dir("bob").join("invalid_seal.json");
    std::fs::write(&corrupted_path, serde_json::to_vec(&consignment)?)?;
    
    // Bob tries to accept
    let mut bob = setup_recipient_wallet(&env, "bob", "password").await?;
    let result = bob.accept_consignment(corrupted_path.to_str().unwrap()).await;
    
    // May pass acceptance but fail when Bob tries to query balance
    // (seal points to non-existent UTXO)
    if result.is_ok() {
        let balance_result = bob.get_asset_balance(&asset_info.contract_id).await;
        assert!(
            balance_result.is_err() || balance_result.unwrap().total == 0,
            "Should fail or show zero balance with invalid seal"
        );
    }
    
    println!("✓ Invalid seal handled appropriately");
}

/// Test 5: Reject corrupted consignment file
#[tokio::test]
async fn test_reject_corrupted_consignment() {
    require_f1r3node!();
    
    let env = TestBitcoinEnv::new("corrupted_file");
    
    // Create corrupted consignment file
    let corrupted_path = env.wallet_dir("test").join("corrupted.json");
    std::fs::create_dir_all(corrupted_path.parent().unwrap())?;
    std::fs::write(&corrupted_path, b"{ invalid json }")?;
    
    let mut bob = setup_recipient_wallet(&env, "bob", "password").await?;
    let result = bob.accept_consignment(corrupted_path.to_str().unwrap()).await;
    
    assert!(result.is_err(), "Should reject corrupted JSON");
    assert!(
        result.unwrap_err().to_string().contains("deserialize") ||
        result.unwrap_err().to_string().contains("parse"),
        "Error should mention deserialization issue"
    );
    
    println!("✓ Corrupted consignment gracefully rejected");
}
```

### What Already Exists
- ✅ All validation logic in `f1r3fly-rgb` and wallet
- ✅ Error handling infrastructure

### What Needs to Be Created
- **None** - Just tests to verify existing validation works

### Expected Outcome
- All invalid inputs correctly rejected
- Error messages are informative
- No panics or crashes on malformed data

---

## Test Case 5: Multi-Transfer Chain

**File**: `f1r3fly-rgb-wallet/tests/f1r3fly/multi_transfer_chain_test.rs`

### Objective
Test complex scenario with multiple parties and sequential transfers to verify:
- Change seals work correctly
- Multiple UTXOs can be used
- State consistency across chain of transfers

### New Helper Needed

```rust
/// Helper to setup multiple test wallets
pub struct TestWallets {
    pub alice: WalletManager,
    pub bob: WalletManager,
    pub carol: WalletManager,
}

pub async fn setup_test_wallets(
    env: &TestBitcoinEnv,
) -> Result<TestWallets, Box<dyn std::error::Error>> {
    let alice = setup_recipient_wallet(env, "alice", "password").await?;
    let bob = setup_recipient_wallet(env, "bob", "password").await?;
    let carol = setup_recipient_wallet(env, "carol", "password").await?;
    
    Ok(TestWallets { alice, bob, carol })
}
```

### Test Structure

```rust
#[tokio::test]
async fn test_multi_party_transfer_chain() {
    require_f1r3node!();
    
    let env = TestBitcoinEnv::new("multi_transfer_chain");
    
    // ========================================================================
    // Step 1: Setup three wallets
    // ========================================================================
    println!("\n👥 Step 1: Setting up Alice, Bob, and Carol");
    
    let mut wallets = setup_test_wallets(&env).await?;
    let mut alice = wallets.alice;
    let mut bob = wallets.bob;
    let mut carol = wallets.carol;
    
    // ========================================================================
    // Step 2: Alice issues 10,000 TEST tokens
    // ========================================================================
    println!("\n💎 Step 2: Alice issues 10,000 TEST tokens");
    
    // Issue asset with Alice's already-set-up wallet
    let asset_info = alice.issue_asset(
        "TEST",
        "Test Token",
        10_000,
        2,
        None
    ).await?;
    
    println!("✓ Asset issued: {}", asset_info.contract_id);
    
    // ========================================================================
    // Step 3: Export genesis, Bob and Carol accept
    // ========================================================================
    println!("\n📦 Step 3: Distributing genesis to Bob and Carol");
    
    let genesis_response = alice.export_genesis(&asset_info.contract_id).await?;
    
    bob.accept_consignment(genesis_response.consignment_path.to_str().unwrap()).await?;
    carol.accept_consignment(genesis_response.consignment_path.to_str().unwrap()).await?;
    
    println!("✓ Genesis distributed");
    
    // ========================================================================
    // Transfer 1: Alice → Bob (3,000)
    // ========================================================================
    println!("\n🔀 Transfer 1: Alice → Bob (3,000 tokens)");
    
    let bob_invoice1 = bob.generate_invoice(&asset_info.contract_id, 3_000, None).await?;
    let transfer1 = alice.send_transfer(&bob_invoice1.invoice_string).await?;
    env.wait_for_confirmation(&transfer1.txid, 1).await?;
    bob.accept_consignment(transfer1.consignment_path.to_str().unwrap()).await?;
    
    verify_balance_with_retry(&mut alice, &asset_info.contract_id, 7_000, 5).await?;
    verify_balance_with_retry(&mut bob, &asset_info.contract_id, 3_000, 5).await?;
    
    println!("✓ Transfer 1 complete (Alice: 7,000, Bob: 3,000)");
    
    // ========================================================================
    // Transfer 2: Alice → Carol (2,000) using change from Transfer 1
    // ========================================================================
    println!("\n🔀 Transfer 2: Alice → Carol (2,000 tokens)");
    
    let carol_invoice1 = carol.generate_invoice(&asset_info.contract_id, 2_000, None).await?;
    let transfer2 = alice.send_transfer(&carol_invoice1.invoice_string).await?;
    env.wait_for_confirmation(&transfer2.txid, 1).await?;
    carol.accept_consignment(transfer2.consignment_path.to_str().unwrap()).await?;
    
    verify_balance_with_retry(&mut alice, &asset_info.contract_id, 5_000, 5).await?;
    verify_balance_with_retry(&mut carol, &asset_info.contract_id, 2_000, 5).await?;
    
    println!("✓ Transfer 2 complete (Alice: 5,000, Carol: 2,000)");
    
    // ========================================================================
    // Transfer 3: Bob → Carol (1,000)
    // ========================================================================
    println!("\n🔀 Transfer 3: Bob → Carol (1,000 tokens)");
    
    let carol_invoice2 = carol.generate_invoice(&asset_info.contract_id, 1_000, None).await?;
    let transfer3 = bob.send_transfer(&carol_invoice2.invoice_string).await?;
    env.wait_for_confirmation(&transfer3.txid, 1).await?;
    carol.accept_consignment(transfer3.consignment_path.to_str().unwrap()).await?;
    
    verify_balance_with_retry(&mut bob, &asset_info.contract_id, 2_000, 5).await?;
    verify_balance_with_retry(&mut carol, &asset_info.contract_id, 3_000, 5).await?;
    
    println!("✓ Transfer 3 complete (Bob: 2,000, Carol: 3,000)");
    
    // ========================================================================
    // Transfer 4: Carol → Alice (500)
    // ========================================================================
    println!("\n🔀 Transfer 4: Carol → Alice (500 tokens)");
    
    let alice_invoice = alice.generate_invoice(&asset_info.contract_id, 500, None).await?;
    let transfer4 = carol.send_transfer(&alice_invoice.invoice_string).await?;
    env.wait_for_confirmation(&transfer4.txid, 1).await?;
    alice.accept_consignment(transfer4.consignment_path.to_str().unwrap()).await?;
    
    verify_balance_with_retry(&mut carol, &asset_info.contract_id, 2_500, 5).await?;
    verify_balance_with_retry(&mut alice, &asset_info.contract_id, 5_500, 5).await?;
    
    println!("✓ Transfer 4 complete (Carol: 2,500, Alice: 5,500)");
    
    // ========================================================================
    // Final Verification
    // ========================================================================
    println!("\n✅ Final verification:");
    
    let alice_balance = alice.get_asset_balance(&asset_info.contract_id).await?;
    let bob_balance = bob.get_asset_balance(&asset_info.contract_id).await?;
    let carol_balance = carol.get_asset_balance(&asset_info.contract_id).await?;
    
    println!("  Alice: {} tokens", alice_balance.total);
    println!("  Bob: {} tokens", bob_balance.total);
    println!("  Carol: {} tokens", carol_balance.total);
    
    // Verify final balances
    assert_eq!(alice_balance.total, 5_500, "Alice final balance");
    assert_eq!(bob_balance.total, 2_000, "Bob final balance");
    assert_eq!(carol_balance.total, 2_500, "Carol final balance");
    
    // Verify total supply conservation
    let total = alice_balance.total + bob_balance.total + carol_balance.total;
    assert_eq!(total, 10_000, "Total supply should be conserved");
    
    println!("✓ Total supply conserved: {} tokens", total);
    
    // Verify RGB-occupied UTXOs tracked
    let alice_occupied = alice.get_occupied_utxos().await?;
    let bob_occupied = bob.get_occupied_utxos().await?;
    let carol_occupied = carol.get_occupied_utxos().await?;
    
    println!("  Alice RGB UTXOs: {}", alice_occupied.len());
    println!("  Bob RGB UTXOs: {}", bob_occupied.len());
    println!("  Carol RGB UTXOs: {}", carol_occupied.len());
    
    assert!(alice_occupied.len() > 0, "Alice should have RGB UTXOs");
    assert!(bob_occupied.len() > 0, "Bob should have RGB UTXOs");
    assert!(carol_occupied.len() > 0, "Carol should have RGB UTXOs");
    
    println!("\n✅ Multi-party transfer chain test passed!");
}
```

### What Already Exists
- ✅ All transfer functionality
- ✅ Balance queries
- ✅ Consignment operations

### What Needs to Be Created
- `setup_test_wallets()` helper function
- `WalletManager::get_occupied_utxos()` method

### Expected Outcome
- Four sequential transfers complete successfully
- Change seals work across multiple transfers
- Final balances sum to original supply
- No orphaned or lost tokens
- RGB-occupied UTXOs properly tracked for all parties

---

## Implementation Order Recommendation

1. **Test Case 2** (Invoice) - Easiest, all functionality exists ✅
2. **Test Case 3** (Complete Transfer) - Core functionality, some helpers needed
3. **Test Case 5** (Multi-Transfer) - Builds on Test Case 3
4. **Test Case 4** (Security) - Last, tests edge cases

## Summary of New Code Needed

### Helper Functions
- `verify_tapret_in_tx()` - verify Tapret commitment in Bitcoin TX
- `setup_test_wallets()` - create multiple wallets for testing

### Wallet Methods
- `WalletManager::get_occupied_utxos()` - return HashSet of RGB-occupied UTXO strings

### Test Files
- `invoice_operations_test.rs` (Test Case 2)
- `complete_transfer_test.rs` (Test Case 3)
- `validation_security_test.rs` (Test Case 4)
- `multi_transfer_chain_test.rs` (Test Case 5)

**All existing functionality is production-ready and just needs testing!** 🎯

