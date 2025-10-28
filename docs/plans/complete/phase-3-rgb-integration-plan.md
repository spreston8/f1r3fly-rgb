# Phase 3: RGB Integration & Unlockable Feature - Implementation Plan

## Overview
This phase integrates the RGB runtime to detect and display RGB assets bound to UTXOs, and implements the "Unlock UTXO" feature that allows users to recover Bitcoin from any UTXO (with appropriate warnings for occupied ones).

**📚 Research Document**: See [RGB Runtime Research Findings](./rgb-runtime-research-findings.md) for detailed technical research and API discoveries.

---

## Part A: RGB Runtime Integration (Detection)

### Backend Changes

**1. Add RGB Dependencies** (`wallet/Cargo.toml`)
- Add RGB runtime crates:
  - `rgb-runtime`
  - `rgb-std`
  - `rgb-core`
  - `bp-std`
  - Others as needed

**2. Create RGB Module** (`wallet/src/wallet/rgb.rs`)
- `RgbManager` struct to interact with RGB runtime
- Initialize RGB runtime with proper data directory
- Methods:
  - `check_utxo_occupied(txid: Txid, vout: u32) -> Result<bool>`
  - `get_bound_assets(txid: Txid, vout: u32) -> Result<Vec<BoundAsset>>`
  - Store RGB state alongside wallet data

**3. Update UTXO Data Structure** (`wallet/src/wallet/balance.rs`)
- Add `bound_assets: Vec<BoundAsset>` to `UTXO` struct
- `BoundAsset` struct:
  ```rust
  pub struct BoundAsset {
      pub asset_id: String,        // Contract ID
      pub asset_name: String,       // e.g., "F1R3FLYTEST1"
      pub ticker: String,           // e.g., "F"
      pub amount: u64,              // Token amount
  }
  ```

**4. Update Balance Checker** (`wallet/src/wallet/balance.rs`)
- After fetching UTXOs from Esplora, query RGB runtime for each UTXO
- Set `is_occupied` and `bound_assets` fields based on RGB data
- Cache results to avoid repeated queries

**5. Update API Types** (`wallet/src/api/types.rs`)
- Ensure `BoundAsset` struct is exposed in API types
- Update serialization/deserialization

---

### Frontend Changes

**6. Update TypeScript Types** (`wallet-frontend/src/api/types.ts`)
```typescript
export interface BoundAsset {
  asset_id: string;
  asset_name: string;
  ticker: string;
  amount: number;
}

export interface UTXO {
  // ... existing fields ...
  is_occupied: boolean;
  bound_assets: BoundAsset[];  // NEW
}
```

**7. Update UTXO Display** (`wallet-frontend/src/components/UTXOList.tsx`)
- For occupied UTXOs, show bound assets below the basic UTXO info
- Display format:
  ```
  ✓ Available / 🔒 RGB Asset
  txid:vout
  30,000 sats | 4 confirmations
  
  [For occupied only:]
  Bound RGB assets:
  • F (F1R3FLYTEST1) - contract...l0zI
  • [more assets if multiple]
  ```
- Truncate contract IDs with tooltip for full ID
- Color-coded asset badges

---

## Part B: Unlockable Tab & Actions

### UI/UX Decision

**Option 1: Replace "All" with "Unlockable"**
- Tabs become: "Unlockable" (all), "Unoccupied", "Occupied"
- More aligned with Bitlight UX
- "Unlockable" implies you can take action on these

**Option 2: Keep "All" and add "Unlockable" as 4th tab**
- Tabs: "All", "Unoccupied", "Occupied", "Unlockable"
- "All" is passive viewing
- "Unlockable" shows same UTXOs but with unlock actions enabled
- Might be redundant

**Recommendation: Option 1** - Replace "All" with "Unlockable" for clearer intent.

---

### Backend Changes

**8. Create Unlock Endpoint** (`wallet/src/wallet/manager.rs`)
- New method: `unlock_utxo(wallet_name, txid, vout) -> Result<UnlockResult>`
- Build transaction:
  - Input: The specified UTXO
  - Output: Send entire amount (minus fee) to a new address in the wallet
- Sign and broadcast transaction
- Return transaction ID and amount recovered

**9. Add API Handler** (`wallet/src/api/handlers.rs`)
- `POST /api/wallet/:name/unlock-utxo`
- Request body: `{ "txid": "...", "vout": 0 }`
- Response: `{ "txid": "...", "recovered_sats": 30000, "fee_sats": 200 }`

**10. Update Transaction Builder** (`wallet/src/wallet/transaction.rs`)
- Add `build_unlock_utxo_tx()` method
- Simple transaction: 1 input, 1 output (to self, minus fee)

---

### Frontend Changes

**11. Rename "All" Tab to "Unlockable"** (`wallet-frontend/src/components/UTXOList.tsx`)
- Change `type TabType = 'all' | 'unoccupied' | 'occupied'` to `type TabType = 'unlockable' | 'unoccupied' | 'occupied'`
- Update tab button text and logic
- Default to "Unlockable" tab on load

**12. Add Unlock Button to Each UTXO** (`wallet-frontend/src/components/UTXOList.tsx`)
- Only show "Unlock" button when on "Unlockable" tab
- Button styled as warning/caution (orange/yellow)
- Position next to the UTXO details
- Different button text/color for occupied vs unoccupied:
  - Unoccupied: "🔓 Unlock" (neutral yellow)
  - Occupied: "⚠️ Unlock (Forfeit Assets)" (danger red)

**13. Create UnlockUtxoModal Component** (`wallet-frontend/src/components/UnlockUtxoModal.tsx`)

**Modal Structure:**

```typescript
interface UnlockUtxoModalProps {
  walletName: string;
  utxo: UTXO;
  isOpen: boolean;
  onClose: () => void;
  onSuccess: () => void;
}
```

**For Unoccupied UTXO:**
```
Unlock UTXO

Output: txid:vout
Amount: 30,000 sats

Notice:
UTXO unlocking requires a transaction fee. After unlocking, 
the available BTC in the original UTXO will be transferred 
to your wallet balance.

Estimated fee: ~200 sats

[Cancel] [Unlock UTXO]
```

**For Occupied UTXO:**
```
⚠️ UNLOCK UTXO - WARNING

Output: txid:vout
Amount: 30,000 sats

Notice:
UTXO unlocking requires a transaction fee. After unlocking, 
the available BTC in the original UTXO will be transferred 
to your wallet balance.

⛔ YOU WILL FORFEIT THESE RGB ASSETS:

• F (F1R3FLYTEST1)
  Contract: rgb1...l0zI
  Amount: 1000 tokens

• [Other assets...]

These assets will be PERMANENTLY LOST and cannot be recovered.

☑ I understand I will lose these RGB assets forever

Estimated fee: ~200 sats

[Cancel] [⚠️ Unlock & Forfeit Assets]
```

**Key UX Elements:**
- Large warning section for occupied UTXOs
- Red color scheme for destructive action
- Checkbox confirmation required for occupied
- "Unlock" button disabled until checkbox is checked
- List all bound assets clearly
- Show full contract IDs (with copy button)

**14. Integrate Modal into UTXOList**
- Clicking "Unlock" button opens the modal
- Pass the selected UTXO to the modal
- Modal determines UI based on `utxo.is_occupied` and `utxo.bound_assets`
- On success, refresh wallet data and close modal

**15. Add API Function** (`wallet-frontend/src/api/wallet.ts`)
```typescript
unlockUtxo: async (
  name: string, 
  txid: string, 
  vout: number
): Promise<UnlockUtxoResponse> => {
  const response = await apiClient.post<UnlockUtxoResponse>(
    `/wallet/${name}/unlock-utxo`, 
    { txid, vout }
  );
  return response.data;
}
```

---

## Part C: Testing & Validation

### Manual Testing Checklist

**RGB Integration:**
- [ ] Create a test RGB asset and bind it to a UTXO
- [ ] Verify `is_occupied` is true for that UTXO
- [ ] Verify `bound_assets` array is populated with correct data
- [ ] Check "Occupied" tab shows the UTXO
- [ ] Verify asset details display correctly (name, ticker, contract ID)

**Unlockable Tab:**
- [ ] Verify "Unlockable" tab shows all UTXOs (occupied + unoccupied)
- [ ] Verify counts match: Unlockable = Unoccupied + Occupied
- [ ] Verify "Unlock" buttons appear on UTXOs in "Unlockable" tab only

**Unlock Unoccupied UTXO:**
- [ ] Click unlock on an unoccupied UTXO
- [ ] Verify modal shows correct notice (no asset forfeit warning)
- [ ] Submit unlock transaction
- [ ] Verify transaction broadcasts successfully
- [ ] Verify Bitcoin is recovered to wallet balance
- [ ] Verify UTXO disappears after confirmation

**Unlock Occupied UTXO:**
- [ ] Click unlock on an occupied UTXO
- [ ] Verify modal shows RED warning with asset forfeit list
- [ ] Verify all bound assets are listed correctly
- [ ] Verify unlock button is disabled until checkbox is checked
- [ ] Submit unlock transaction
- [ ] Verify transaction broadcasts successfully
- [ ] Verify Bitcoin is recovered to wallet balance
- [ ] Verify RGB assets are gone (check RGB runtime)

---

## Part D: Edge Cases & Considerations

### 1. RGB Runtime Initialization
- Where to store RGB data directory? (same as wallet storage)
- How to initialize RGB runtime on server startup?
- Handle case where RGB data is not initialized yet

### 2. Performance
- Querying RGB runtime for every UTXO could be slow
- Implement caching layer for occupied status
- Consider background job to update UTXO occupation status

### 3. Error Handling
- RGB runtime errors (stash not found, corrupted data)
- Transaction broadcast failures for unlock
- Insufficient funds for fee (though this should be rare)

### 4. Multiple Assets on One UTXO
- A UTXO can have multiple RGB assets bound to it
- UI must handle displaying multiple assets gracefully
- Modal must list ALL assets that will be forfeited

### 5. Partially Confirmed UTXOs
- Should unconfirmed UTXOs be unlockable?
- Probably yes, but show confirmation count as warning

### 6. Fee Estimation for Unlock
- Estimate fee before transaction is built
- Show estimated fee in modal
- Allow user to adjust fee rate (advanced option)

---

## Implementation Order

**Step 1:** RGB Runtime Integration (Backend)
- Add RGB dependencies
- Create RgbManager module
- Implement UTXO occupation detection
- Update balance fetching to include RGB data

**Step 2:** Display Bound Assets (Frontend)
- Update types to include BoundAsset
- Modify UTXOList to display asset information
- Style occupied UTXOs with asset badges

**Step 3:** Rename Tab (Frontend)
- Change "All" to "Unlockable"
- Update tab logic and styling

**Step 4:** Unlock Transaction (Backend)
- Implement `unlock_utxo` method
- Add API endpoint
- Test transaction building and broadcasting

**Step 5:** Unlock Modal (Frontend)
- Create UnlockUtxoModal component
- Implement conditional UI (occupied vs unoccupied)
- Add confirmation checkbox for occupied UTXOs

**Step 6:** Integration & Testing
- Wire up unlock button to modal
- Test all scenarios
- Handle errors gracefully

---

## Open Questions

1. **RGB Runtime Configuration:** Which RGB libraries should we use? (rgb-runtime, rgb-std, etc.)
2. **Asset Display:** Should we show token amounts for all assets, or just presence?
3. **Fee Rate:** Fixed fee rate or user-configurable for unlock transactions?
4. **Unlock All:** Should we add "Unlock All" feature for batch operations?
5. **Transaction History:** Should unlocked UTXOs be tracked separately in history?

---

## Summary

Phase 3 transforms the wallet from a Bitcoin-only interface to a full RGB-aware wallet:

✅ Detects RGB assets on UTXOs via runtime integration  
✅ Displays bound assets clearly on occupied UTXOs  
✅ Provides "Unlockable" tab aligned with Bitlight UX  
✅ Implements safe unlock mechanism with appropriate warnings  
✅ Handles both simple (unoccupied) and dangerous (occupied) unlock scenarios  

This phase requires both significant backend work (RGB integration) and frontend UX work (modals, warnings, asset display).

