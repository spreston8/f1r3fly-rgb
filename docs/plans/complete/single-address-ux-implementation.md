# Single Address UX Implementation Plan

## Overview
Provide users with ONE visible address while maintaining backend address separation for RGB and Bitcoin operations.

---

## Phase 1: State Management

**Update `state.json`:**
```json
{
  "public_address_index": 0,           // Always 0, shown to user
  "internal_next_index": 1,            // For change/internal, hidden
  "last_synced_height": 274195
}
```

**Key concepts:**
- `public_address_index`: Fixed at 0, user-facing address for all receives
- `internal_next_index`: Auto-increments for change outputs, backend only

**Files to modify:**
- `wallet/src/wallet/shared/storage.rs` - Add state fields
- `wallet/src/api/types.rs` - Update WalletInfo structure

---

## Phase 2: How Current "Create UTXO" Pattern Fits

**Current flow:**
```rust
// User clicks "Create UTXO" button
fn create_utxo() {
    // Creates a UTXO for RGB operations
}
```

**New flow:**
```rust
// User clicks "Create UTXO" (or auto-triggered)
fn create_utxo_for_rgb() {
    const PUBLIC_INDEX: u32 = 0;
    
    // Create UTXO at index 0 (public address)
    create_utxo_at_index(PUBLIC_INDEX, 10_000)?;
    
    // User sees: "UTXO created at your wallet address"
}

// When RGB needs a UTXO for invoice
fn generate_rgb_invoice() {
    const PUBLIC_INDEX: u32 = 0;
    
    // Check if UTXO exists at index 0
    if !has_available_utxo_at(PUBLIC_INDEX) {
        create_utxo_at_index(PUBLIC_INDEX, 10_000)?;
    }
    
    // Generate invoice using UTXO at index 0
}
```

**Integration:**
- ✅ Keeps "Create UTXO" feature
- ✅ Ensures it creates at index 0
- ✅ User doesn't see index numbers

---

## Phase 3: Address Usage Pattern

### Index 0 (Public - User Sees This)

**Used for:**
1. RGB invoice generation (seal UTXO)
2. RGB payment receipts (tokens arrive here)
3. Regular Bitcoin receives (user shares this address)
4. Initial Bitcoin funding (user deposits here)

**Frontend shows:**
```
Your Address: tb1q... (index 0)
Send Bitcoin or RGB tokens to this address
```

### Index 1+ (Internal - User Never Sees)

**Used for:**
1. Bitcoin transaction change outputs
2. Internal UTXO management
3. RGB transfer change
4. Automatic wallet operations

**Frontend shows:**
```
(nothing - completely hidden)
```

---

## Phase 4: Balance Aggregation

**Backend calculates total from ALL indices:**

```rust
pub async fn get_balance(
    storage: &Storage,
    wallet_name: &str,
) -> Result<BalanceInfo, WalletError> {
    let descriptor = storage.load_descriptor(wallet_name)?;
    
    // Scan ALL addresses (0, 1, 2, 3...) using gap limit
    let all_addresses = derive_addresses_with_gap_limit(&descriptor)?;
    
    // Query Bitcoin balance from ALL addresses
    let mut total_bitcoin = 0u64;
    for (index, address) in all_addresses {
        let balance = query_address_balance(address).await?;
        total_bitcoin += balance;
        log::debug!("Address index {}: {} sats", index, balance);
    }
    
    // Query RGB balance (aggregates from all addresses)
    let rgb_balances = query_rgb_balances(wallet_name)?;
    
    // Return aggregated view
    Ok(BalanceInfo {
        bitcoin_balance: total_bitcoin,      // Sum of all indices
        rgb_assets: rgb_balances,            // All RGB from all indices
        display_address: derive_address(0),  // Only show index 0
    })
}
```

**User sees:**
- ✅ One address: `tb1q...` (index 0)
- ✅ Total balance: Aggregated from all indices
- ✅ Total RGB: All tokens from all indices

---

## Phase 5: Transaction Construction

### Scenario 1: User Sends Bitcoin

```rust
fn send_bitcoin(to_address: &str, amount: u64) {
    // 1. Collect UTXOs from ALL indices (0, 1, 2, 3...)
    let available_utxos = scan_all_addresses_for_utxos()?;
    
    // 2. Select UTXOs for payment
    let selected = select_coins(available_utxos, amount)?;
    
    // 3. Build transaction with change to internal index
    let state = load_state()?;
    let change_index = state.internal_next_index;
    let change_address = derive_address(change_index)?;
    
    let tx = build_transaction(
        inputs: selected,
        outputs: [
            (to_address, amount),
            (change_address, change_amount),  // Internal index
        ]
    )?;
    
    // 4. Update state
    state.internal_next_index += 1;
    save_state(state)?;
    
    // 5. Sign and broadcast
    sign_and_broadcast(tx)?;
}
```

**Result:**
- User doesn't see change address
- Next balance query aggregates all indices
- User sees updated total balance

### Scenario 2: User Sends RGB Token

```rust
fn send_rgb_transfer(invoice: &str) {
    const PUBLIC_INDEX: u32 = 0;
    
    // 1. RGB transfer uses UTXO from index 0
    let utxo_at_public = get_utxo_at_index(PUBLIC_INDEX)?;
    
    // 2. Build RGB transfer
    let psbt = create_rgb_transfer(invoice, utxo_at_public)?;
    
    // 3. If there's Bitcoin change, send to internal index
    let state = load_state()?;
    if has_change_output(&psbt) {
        let change_index = state.internal_next_index;
        set_change_address(&mut psbt, derive_address(change_index)?)?;
        state.internal_next_index += 1;
    }
    
    // 4. Sign and broadcast
    sign_and_broadcast(psbt)?;
}
```

---

## Phase 6: Frontend Implementation

**Single Address Display:**

```tsx
// WalletView.tsx
<Card>
  <CardHeader>
    <CardTitle>Your Wallet Address</CardTitle>
  </CardHeader>
  <CardContent>
    {/* Only show public address (index 0) */}
    <div className="address-display">
      <QRCode value={wallet.address} />
      <code>{wallet.address}</code>
      <Button onClick={copyAddress}>Copy</Button>
    </div>
    
    <p className="text-sm text-muted">
      Use this address to receive Bitcoin and RGB tokens
    </p>
  </CardContent>
</Card>

{/* Balance - aggregated from all backend addresses */}
<Card>
  <CardHeader>
    <CardTitle>Balance</CardTitle>
  </CardHeader>
  <CardContent>
    <div className="balance">
      <div>Bitcoin: {formatSats(wallet.bitcoin_balance)}</div>
      {wallet.rgb_assets.map(asset => (
        <div key={asset.contract_id}>
          {asset.name}: {asset.balance}
        </div>
      ))}
    </div>
  </CardContent>
</Card>
```

**What user NEVER sees:**
- ❌ "Index 0, Index 1, Index 2..."
- ❌ Multiple addresses
- ❌ "Change addresses"
- ❌ "Internal addresses"

**What user DOES see:**
- ✅ ONE address
- ✅ Aggregated balance
- ✅ Simple UX

---

## Phase 7: Gap Limit Strategy

**Configuration:**

```rust
// In rgb/src/owner.rs
let max_empty_batches = 1;   // Standard
let min_scan_index = 20;     // Standard

// Scans: 0-40 addresses
// Finds:
//   - Index 0: Public (user's address)
//   - Index 1-N: Internal addresses (change, automatic)
// Time: ~12-15 seconds
```

**Why this works:**
- Index 0 always used (public address)
- Indices 1+ sequential (change outputs)
- No gaps (change outputs created sequentially)
- Gap limit 20 covers normal usage

**After 100 operations:**
- Indices 0-100 used
- Dynamic gap limit: `max(20, internal_next_index)`
- Always fast enough

---

## User Experience Summary

### User Perspective:

**Onboarding:**
1. Create wallet
2. See ONE address: tb1q...
3. Copy address to receive funds

**Sending Bitcoin:**
1. Click "Send Bitcoin"
2. Enter recipient + amount
3. Confirm transaction
4. Balance updates (aggregated)

**RGB Operations:**
1. Click "Generate Invoice"
2. Invoice shows same address: tb1q...
3. Receive tokens at that address
4. Balance updates

**What they NEVER deal with:**
- ❌ Choosing which address to use
- ❌ Managing multiple addresses
- ❌ Understanding indices
- ❌ Worrying about change addresses

### Backend Reality:

```
Index 0: tb1q... (public)
├─ Receives: User deposits, RGB payments
├─ Balance: 50,000 sats + 100 RGB tokens
└─ Shown to user: YES

Index 1: tb1p... (internal)
├─ Created: Auto (change from TX #1)
├─ Balance: 5,000 sats
└─ Shown to user: NO

Index 2: tb1p... (internal)
├─ Created: Auto (change from TX #2)  
├─ Balance: 12,000 sats
└─ Shown to user: NO

Total displayed: 67,000 sats (aggregated)
```

---

## Benefits

✅ **Simple UX**: One address for everything  
✅ **Clean separation**: RGB and Bitcoin properly managed  
✅ **Full balance**: Aggregated from all addresses  
✅ **Fast sync**: Gap limit 20 works forever  
✅ **Scalable**: Handles unlimited operations  
✅ **Hidden complexity**: Backend details invisible to user

