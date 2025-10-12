# RGB Transfer User Flows

**Status**: Production Implementation Complete  
**Date**: October 12, 2025  
**Phases Complete**: 2 (Invoice), 3 (Send), 4A (Genesis Export), 4B (Accept Consignment)

---

## Table of Contents

1. [Overview](#overview)
2. [Flow 1: Same Wallet Sync (Genesis)](#flow-1-same-wallet-sync-genesis-)
3. [Flow 2: Transfer Between Wallets](#flow-2-transfer-between-wallets-)
4. [Comparison Table](#comparison-table-)
5. [Current UI Elements](#current-ui-elements-)
6. [Technical Flow](#technical-flow-behind-the-scenes-)
7. [Error Scenarios](#error-scenarios-)
8. [Best Practices](#best-practices-)

---

## Overview

This document describes the complete user flows for RGB asset transfers in the f1r3fly wallet. There are two distinct flows:

1. **Genesis Sync** - Share contract state across devices with the same wallet (no Bitcoin TX)
2. **Token Transfer** - Send tokens from one wallet to another (requires Bitcoin TX)

Both flows use RGB consignments but serve different purposes and have different requirements.

---

## Flow 1: Same Wallet Sync (Genesis) 🔄

**Scenario**: User issues asset on Computer A, wants to see it on Computer B (same wallet mnemonic)

**Purpose**: Share contract knowledge across devices  
**Wallets**: Same (same mnemonic phrase)  
**Bitcoin TX**: ❌ Not required  
**Time**: Instant  
**Cost**: Free

---

### Computer A (Issuer)

#### Step 1: Issue Asset

1. Navigate to wallet detail page
2. Click **"Issue RGB Asset"** button
3. Fill in asset details:
   - Ticker symbol (e.g., "TOKEN")
   - Asset name (e.g., "My Token")
   - Total supply (e.g., "1000000")
   - Precision/decimals (e.g., "8" for Bitcoin-like)
4. Select genesis UTXO from available UTXOs
5. Click **"Issue Asset"**
6. Asset appears in **RGB Assets** section with balance

#### Step 2: Export Genesis Consignment

1. Locate the asset in RGB Assets list
2. Click **📦 Export** button (purple button)
3. **Export Genesis Modal** opens:
   - Shows explanation: "📱 Sync wallet across devices"
   - Displays asset name and contract ID
   - Note: "No Bitcoin transaction required"
4. Click **📤 Export Genesis** button
5. Backend processing:
   - Validates contract exists
   - Checks for allocations
   - Creates `exports/genesis_<contract_id>.rgbc`
   - Uses empty terminals (genesis-only, no transfer)
6. Success state shows:
   - File size (e.g., "12.5 KB")
   - Filename: `genesis_<contract_id>.rgbc`
7. Click **📥 Download File** button
8. File saved to downloads folder

#### Step 3: Transfer File to Computer B

Transfer the `.rgbc` file using any method:
- **USB Drive**: Copy to USB, physically move to Computer B
- **Cloud Storage**: Upload to Dropbox, Google Drive, iCloud
- **Network Share**: Local network file sharing
- **Email**: Attach to email (small file size)
- **Secure Messaging**: Signal, Telegram file sharing

---

### Computer B (Same Wallet)

#### Step 1: Open Wallet

1. Launch wallet application
2. Import wallet using **exact same mnemonic** as Computer A
3. Wallet loads successfully
4. RGB Assets section shows no assets (expected)
5. Bitcoin balance and UTXOs are synced

#### Step 2: Import Genesis Consignment

1. Click **📥 Import Consignment** button (top of wallet page)
2. **Accept Consignment Modal** opens:
   - Explanation of genesis vs transfer
   - File upload area
3. Click **"Select Consignment File"**
4. Browse to `genesis_<contract_id>.rgbc` file
5. File selected, shows filename and size
6. Click **Import Consignment** button

#### Step 3: Backend Processing

Backend automatically:
1. Saves file to temp directory
2. Gets current contract IDs
3. Validates consignment: `runtime.consume_from_file()`
4. Compares contract IDs to find new contracts
5. Queries witnesses: `runtime.contracts.contract_witnesses()`
6. Detects: **0 witnesses** → `import_type: "genesis"`
7. No Bitcoin TX to check
8. Returns: `status: "genesis_imported"`

#### Step 4: Result Display

Success modal shows:
- ✅ "Consignment imported successfully!"
- **Type**: 🎁 Genesis (blue badge)
- **Status**: 🎁 Genesis Imported (blue badge)
- **Contract ID**: Full contract ID displayed
- **Bitcoin TX**: (none - not shown for genesis)
- Tip: "Sync your wallet to see updated token balances"

#### Step 5: Sync and Verify

1. Click **"Sync Wallet"** button (top of page)
2. Wallet refreshes data
3. Asset now appears in **RGB Assets** section:
   - Same ticker and name as Computer A
   - Same balance as Computer A
   - **Same UTXO** as Computer A (e.g., `abc123...def456:0`)
4. UTXO shows as "Occupied by RGB assets"

**Result**: ✅ Both computers now see the same asset on the same UTXO

---

### Genesis Sync Key Points

- ✅ **No Bitcoin transaction required** - just sharing knowledge
- ✅ **Same UTXO on both devices** - not creating new UTXOs
- ✅ **Instant sync** - no waiting for confirmations
- ✅ **Free** - no network fees
- ✅ **Same wallet required** - same mnemonic phrase must be used
- ✅ **Reusable file** - same genesis file works for multiple devices
- ⚠️ **Not for sending tokens** - use token transfer flow instead

---

## Flow 2: Transfer Between Wallets 💸

**Scenario**: Alice sends 100 tokens to Bob (different wallets with different mnemonics)

**Purpose**: Transfer token ownership  
**Wallets**: Different (different mnemonic phrases)  
**Bitcoin TX**: ✅ Required  
**Time**: ~10 minutes (block confirmation)  
**Cost**: Bitcoin network fee (~250-500 sats)

---

### Bob (Recipient) - Generate Invoice

#### Step 1: Request Tokens

1. Open wallet, navigate to wallet detail page
2. Locate RGB asset in **RGB Assets** section
3. Click **📨 Receive** button (green button)
4. **Generate Invoice Modal** opens

#### Step 2: Generate Invoice

1. Enter amount to receive:
   - Input: `100` (tokens)
2. Click **Generate Invoice** button
3. Backend processing:
   - RGB runtime syncs: `runtime.update(1)`
   - Gets available UTXO for seal: `runtime.auth_token(0)`
   - Creates blinded seal from UTXO
   - Generates invoice: `RgbInvoice::new(contract_id, beneficiary, amount)`
   - Returns invoice string

#### Step 3: Success and Share

Success state shows:
- ✅ "Invoice Generated"
- **Invoice String**: Full RGB invoice (starts with `rgb:`)
- **Amount**: 100 tokens
- **Seal UTXO**: Shows which UTXO will receive (e.g., `xyz789:1`)
- **Copy button**: One-click copy to clipboard

1. Click **Copy** button
2. Share invoice with Alice via:
   - **Secure messaging**: Signal, Telegram, WhatsApp
   - **Email**: Paste invoice string
   - **QR code**: (future feature)
3. Keep modal open or save invoice string

---

### Alice (Sender) - Send Transfer

#### Step 1: Prepare Transfer

1. Open wallet, navigate to wallet detail page
2. Locate RGB asset with sufficient balance
3. Click **📤 Send** button (blue button)
4. **Send Transfer Modal** opens

#### Step 2: Enter Transfer Details

1. Paste Bob's invoice string into **Invoice** field
2. (Optional) Adjust fee rate:
   - Default: 1 sat/vB
   - Higher = faster confirmation
3. Click **Send Transfer** button

#### Step 3: Backend Processing (Complex)

Backend performs multiple steps:

1. **Parse Invoice**
   - `RgbInvoice::from_str(invoice)`
   - Validates format and extracts data

2. **Initialize Runtime**
   - `get_runtime(wallet_name)`
   - Loads RGB wallet and contracts

3. **Create Payment (PSBT)**
   - `runtime.pay_invoice(&invoice, strategy, tx_params, min_lock)`
   - Selects UTXOs with RGB balance
   - Creates Bitcoin transaction structure
   - **DBC commit done automatically** (tapret commitment)
   - Returns: `(psbt, payment)` with terminals

4. **Generate Consignment (BEFORE signing!)**
   - Creates: `consignments/transfer_<contract_id>_<timestamp>.rgbc`
   - `runtime.contracts.consign_to_file(path, contract_id, payment.terminals)`
   - Contains:
     - Full contract state history
     - Genesis operation
     - All state transitions
     - New allocation to Bob's blinded seal
     - Proof chain

5. **Sign PSBT**
   - `psbt.sign(&WalletSigner)`
   - Uses BIP84 key derivation (m/84'/1'/0'/0/x)
   - Signs each input with correct private key
   - ECDSA signatures for P2WPKH

6. **Finalize PSBT**
   - `psbt.finalize(descriptor)`
   - Completes witness scripts
   - Prepares for broadcast

7. **Extract Transaction**
   - `psbt.extract()` → signed Bitcoin transaction
   - Converts to hex format: `format!("{:x}", tx)`
   - Gets TX ID: `tx.txid()`

8. **Broadcast Transaction**
   - POST to Esplora API: `https://mempool.space/signet/api/tx`
   - Sends hex transaction
   - Returns Bitcoin TX ID

#### Step 4: Success and Download

Success modal shows:
- ✅ "Transfer Broadcasted"
- **Bitcoin TX ID**: Clickable link to mempool.space
  - Example: `a1b2c3d4...` → opens explorer
- **Consignment Filename**: `transfer_<contract_id>_<timestamp>.rgbc`
- **Status**: "broadcasted"
- **Download Button**: 📥 Download Consignment

1. Click **📥 Download Consignment** button
2. File saved to downloads folder
3. **IMPORTANT**: Must send this file to Bob!

#### Step 5: Share Consignment with Bob

Send the `.rgbc` file to Bob using:
- **Secure Messaging**: Signal, Telegram (preferred)
- **Email**: Attach file
- **File Sharing**: Dropbox, Google Drive link
- **Direct Transfer**: If in person

**Critical**: Bob cannot receive tokens without this file!

---

### Bob (Recipient) - Accept Consignment

#### Step 1: Receive File

1. Download `transfer_<contract_id>_<timestamp>.rgbc` from Alice
2. Save to known location on computer
3. Open wallet application

#### Step 2: Import Transfer Consignment

1. Click **📥 Import Consignment** button (top of wallet page)
2. **Accept Consignment Modal** opens
3. Click **"Select Consignment File"**
4. Browse to `transfer_*.rgbc` file
5. File selected, shows filename and size (larger than genesis)
6. Click **Import Consignment** button

#### Step 3: Backend Processing

Backend automatically:
1. Saves file to temp directory
2. Gets current contract IDs (before import)
3. Validates consignment: `runtime.consume_from_file()`
   - Verifies all signatures
   - Validates state transitions
   - Checks DBC commitments
4. Compares contract IDs to find new contracts
5. Queries witnesses: `runtime.contracts.contract_witnesses(contract_id)`
6. Detects: **Has witnesses** → `import_type: "transfer"`
7. Extracts Bitcoin TX ID: `witness.id.to_string()`
8. Checks TX status:
   - `WitnessStatus::Tentative` → "pending"
   - `WitnessStatus::Mined(_)` → "confirmed"
9. Returns full metadata

#### Step 4: Result Display

Success modal shows:
- ✅ "Consignment imported successfully!"
- **Type**: 💸 Transfer (purple badge)
- **Status**: 
  - ⏳ Pending (yellow badge) - if TX unconfirmed
  - ✅ Confirmed (green badge) - if TX confirmed
- **Contract ID**: Full contract ID
- **Bitcoin TX**: Clickable link to mempool.space
  - Shows TX ID: `a1b2c3d4...`
  - Opens blockchain explorer
- Message: "Assets will appear after confirmation"

#### Step 5: Wait for Confirmation

1. Click Bitcoin TX link to watch confirmation
2. Signet network: ~10 minutes per block
3. After 1 confirmation:
   - TX status changes to "confirmed"
   - Witness status: `Mined(height)`

#### Step 6: Sync and Verify

1. Click **"Sync Wallet"** button
2. Wallet refreshes data
3. Asset appears/updates in **RGB Assets** section:
   - Balance increased by 100 tokens
   - New UTXO shown (different from Alice's)
4. Check Alice's wallet:
   - Balance decreased by 100 tokens
   - Original UTXO no longer has those tokens

**Result**: ✅ 100 tokens successfully transferred from Alice to Bob

---

### Transfer Key Points

- ✅ **Bitcoin transaction required** - tokens move on-chain
- ✅ **Different UTXOs** - Bob receives on new UTXO
- ✅ **Consignment required** - Bob must receive the file
- ✅ **Client-side validation** - Bob verifies independently
- ✅ **Privacy preserved** - only Alice and Bob know about transfer
- ⏱️ **Takes time** - wait for Bitcoin confirmation (~10 min)
- 💰 **Costs fees** - Bitcoin network fee required
- ⚠️ **One-time use** - consignment is unique per transfer
- ⚠️ **File must be shared** - no automatic relay (yet)

---

## Comparison Table 📊

| Aspect | Genesis Sync | Token Transfer |
|--------|--------------|----------------|
| **Use Case** | Same wallet, different devices | Different wallets, send tokens |
| **Purpose** | Share contract knowledge | Transfer ownership |
| **Mnemonic** | Same on both devices | Different wallets |
| **Bitcoin TX** | ❌ None | ✅ Required |
| **Invoice** | ❌ Not needed | ✅ Required |
| **File Type** | `genesis_*.rgbc` | `transfer_*.rgbc` |
| **File Size** | Smaller (~10-20 KB) | Larger (~50-100 KB) |
| **UTXO** | Same on both devices | Different UTXOs |
| **Balance** | Unchanged | Sender ↓, Recipient ↑ |
| **Time** | Instant | ~10 min (Bitcoin confirm) |
| **Cost** | Free | Bitcoin network fee |
| **Status** | "genesis_imported" | "pending" → "confirmed" |
| **Type Badge** | 🎁 Genesis (blue) | 💸 Transfer (purple) |
| **Witnesses** | 0 | 1+ |
| **Reusable** | ✅ Yes | ❌ No (one-time) |
| **Network** | No blockchain interaction | Blockchain required |

---

## Current UI Elements 🎨

### Wallet Detail Page

**Top Section:**
- **📥 Import Consignment** button (green)
  - Accepts both genesis and transfer consignments
  - Auto-detects type based on witnesses

**RGB Assets Section:**

Each asset displays:
- Asset name and ticker
- Balance
- Contract ID
- Three action buttons:

1. **📤 Send** (blue button)
   - Opens SendTransferModal
   - For sending tokens to another wallet
   - Requires recipient's invoice

2. **📨 Receive** (green button)
   - Opens GenerateInvoiceModal
   - Creates invoice for receiving tokens
   - Generates blinded seal

3. **📦 Export** (purple button)
   - Opens ExportGenesisModal
   - Exports genesis consignment
   - For same-wallet device sync

---

### Modals

#### 1. GenerateInvoiceModal
**Purpose**: Create invoice to receive tokens

**Fields:**
- Amount input (required)
- Asset info display (read-only)

**Actions:**
- Generate Invoice
- Copy invoice string
- Close

**Output:**
- RGB invoice string (starts with `rgb:`)
- Seal UTXO information
- Amount confirmation

---

#### 2. SendTransferModal
**Purpose**: Send tokens to another wallet

**Fields:**
- Invoice input (paste recipient's invoice)
- Fee rate selector (optional, default: 1 sat/vB)

**Actions:**
- Send Transfer
- Download consignment
- Close

**Output:**
- Bitcoin TX ID (clickable)
- Consignment download link
- Broadcast status

---

#### 3. ExportGenesisModal
**Purpose**: Export genesis for same-wallet sync

**Display:**
- Asset name
- Contract ID
- Usage explanation
- Steps for cross-device sync

**Actions:**
- Export Genesis
- Download file
- Close

**Output:**
- Genesis consignment file
- File size
- Download link

---

#### 4. AcceptConsignmentModal
**Purpose**: Import any consignment (auto-detects type)

**Fields:**
- File upload (drag/drop or browse)

**Display After Import:**
- Import type (🎁 Genesis or 💸 Transfer)
- Status badge (with color coding)
- Contract ID
- Bitcoin TX (if transfer)

**Actions:**
- Import Consignment
- Close

**Validation:**
- File format check
- Consignment validation
- State verification

---

## Technical Flow (Behind the Scenes) 🔧

### Send Transfer Technical Steps

```
Input: Invoice string, fee rate
Output: Bitcoin TX ID, consignment file

1. Parse Invoice
   └─→ RgbInvoice::from_str(invoice)
   └─→ Extract: contract_id, beneficiary, amount

2. Initialize Runtime
   └─→ get_runtime(wallet_name)
   └─→ Load: contracts, wallet, resolver

3. Pay Invoice (Create PSBT)
   └─→ runtime.pay_invoice(&invoice, strategy, tx_params, min_lock)
   └─→ Select UTXOs with RGB balance
   └─→ Create Bitcoin transaction structure
   └─→ **DBC commit done internally**
   └─→ Return: (PSBT, Payment with terminals)

4. Generate Consignment (BEFORE SIGNING!)
   └─→ Create file: consignments/transfer_<id>_<time>.rgbc
   └─→ runtime.contracts.consign_to_file(path, id, terminals)
   └─→ Contains:
       ├─ Genesis operation
       ├─ All state transitions
       ├─ Anchor proofs
       ├─ New allocation (to beneficiary)
       └─ Full history chain

5. Sign PSBT
   └─→ psbt.sign(&WalletSigner)
   └─→ For each input:
       ├─ Find UTXO with address_index
       ├─ Derive key: m/84'/1'/0'/0/{index}
       ├─ Create sighash (P2WPKH)
       ├─ Sign with ECDSA
       └─ Add witness data

6. Finalize PSBT
   └─→ psbt.finalize(descriptor)
   └─→ Complete witness scripts
   └─→ Check all inputs signed

7. Extract Transaction
   └─→ psbt.extract() → bpstd::Tx
   └─→ Format hex: format!("{:x}", tx)
   └─→ Get TX ID: tx.txid()

8. Broadcast Transaction
   └─→ POST https://mempool.space/signet/api/tx
   └─→ Body: raw hex transaction
   └─→ Response: TX accepted to mempool

9. Return Result
   └─→ Bitcoin TX ID
   └─→ Consignment download URL
   └─→ Status: "broadcasted"
```

---

### Accept Consignment Technical Steps

```
Input: Consignment file bytes
Output: Import metadata (type, status, TX ID)

1. Save Temp File
   └─→ temp_consignments/accept_<uuid>.rgbc
   └─→ Write bytes to disk

2. Get Current State
   └─→ runtime.contracts.contract_ids()
   └─→ Store in HashSet (before import)

3. Validate and Import
   └─→ runtime.consume_from_file(allow_unknown=true, path)
   └─→ Validates:
       ├─ All signatures
       ├─ State transitions
       ├─ DBC commitments
       ├─ Anchor proofs
       └─ Merkle paths
   └─→ Imports to local stockpile

4. Find New Contracts
   └─→ runtime.contracts.contract_ids()
   └─→ Store in HashSet (after import)
   └─→ Difference = newly imported contracts
   └─→ Get first new contract ID

5. Query Witnesses
   └─→ runtime.contracts.contract_witness_count(id)
   └─→ If count == 0:
       └─→ Type: "genesis"
   └─→ If count > 0:
       └─→ Type: "transfer"
       └─→ Get witnesses: contract_witnesses(id)
       └─→ Last witness = most recent

6. Extract TX Info (if transfer)
   └─→ witness.id → Bitcoin Txid
       └─→ For TxoSeal, id IS Txid directly
   └─→ witness.status → WitnessStatus enum
       ├─ Genesis → "genesis_imported"
       ├─ Offchain → "offchain"
       ├─ Tentative → "pending"
       ├─ Mined(_) → "confirmed"
       └─ Archived → "archived"

7. Cleanup
   └─→ Delete temp file
   └─→ Remove from temp_consignments/

8. Return Result
   └─→ contract_id: String
   └─→ import_type: "genesis" | "transfer"
   └─→ status: String (mapped from WitnessStatus)
   └─→ bitcoin_txid: Option<String> (None for genesis)
```

---

### Export Genesis Technical Steps

```
Input: Contract ID
Output: Genesis consignment file

1. Parse Contract ID
   └─→ ContractId::from_str(contract_id_str)
   └─→ Validate format

2. Initialize Runtime
   └─→ get_runtime(wallet_name)
   └─→ Load contracts

3. Verify Contract Exists
   └─→ runtime.contracts.has_contract(id)
   └─→ Error if not found

4. Check Allocations
   └─→ runtime.contracts.contract_state(id)
   └─→ Verify state.owned is not empty
   └─→ Error if no allocations

5. Create Export Directory
   └─→ mkdir -p exports/
   └─→ Path: exports/genesis_<contract_id>.rgbc

6. Export Genesis Consignment
   └─→ Empty terminals (no new seals)
       └─→ Vec::new() as terminals
   └─→ runtime.contracts.consign_to_file(path, id, terminals)
   └─→ Exports:
       ├─ Contract definition
       ├─ Genesis operation
       ├─ Current state
       └─ Original allocations

7. Get File Metadata
   └─→ File size in bytes
   └─→ Filename

8. Return Result
   └─→ contract_id: String
   └─→ consignment_filename: String
   └─→ file_size_bytes: u64
   └─→ download_url: /api/genesis/{filename}
```

---

## Error Scenarios 🚨

### Genesis Sync Errors

#### Different Mnemonic
**Problem**: Computer B uses different mnemonic than Computer A

**Error**: Asset imports but cannot be used
- Genesis consignment imports successfully
- Asset appears in list
- **But**: Cannot create invoices or send (wrong keys)

**Solution**: Use exact same mnemonic on both devices

---

#### File Corrupted
**Problem**: Genesis file damaged during transfer

**Error**: "Validation failed: Invalid consignment format"

**Solution**: Re-export genesis from Computer A

---

#### Contract Already Exists
**Problem**: Genesis already imported previously

**Error**: "No new contract found after import"

**Solution**: Check if asset already exists in list

---

### Transfer Errors

#### Invalid Invoice Format
**Problem**: Invoice string corrupted or incomplete

**Error**: "Invalid invoice: Parse error"

**Example**: Missing characters, wrong format

**Solution**: Request new invoice from recipient

---

#### Insufficient Balance
**Problem**: Trying to send more tokens than owned

**Error**: "Payment failed: Insufficient RGB balance"

**Solution**: Check balance, send smaller amount

---

#### No Available UTXOs for Seal
**Problem**: No confirmed UTXOs available for recipient seal

**Error**: "No unspent outputs available for seal"

**Solution**: 
- Create new UTXO
- Wait for existing UTXO to confirm
- Unlock occupied UTXO

---

#### Network/Broadcast Failure
**Problem**: Cannot connect to Esplora or broadcast fails

**Error**: "Network error: Broadcast failed"

**Causes**:
- Network connectivity issues
- Esplora server down
- Invalid transaction (rare)

**Solution**:
- Check internet connection
- Wait and retry
- Verify UTXO states

---

#### Wrong Recipient Accepts Consignment
**Problem**: Bob sends Alice's consignment to Carol

**Error**: Import succeeds but tokens are unspendable

**Result**: 
- ✅ Import shows "success"
- ✅ Balance shows increased
- ❌ Cannot create invoices (wrong blinded seal)
- ❌ Cannot send tokens (not her keys)

**Prevention**: Verify sender before importing transfer consignments

---

#### Consignment File Lost
**Problem**: Sender doesn't download/share consignment file

**Error**: Recipient never receives tokens

**Result**:
- Bitcoin TX confirms on blockchain
- Tokens leave sender's wallet
- Recipient cannot claim tokens without consignment

**Prevention**: Always download consignment before closing modal

---

## Best Practices 👍

### For All Users

1. **Test on Signet First**
   - Use test network before mainnet
   - Familiarize with workflows
   - Verify understanding of genesis vs transfer

2. **Keep Consignment Files**
   - Store as proof of transfer
   - Backup in secure location
   - May need for auditing or verification

3. **Verify Before Importing**
   - Check sender identity
   - Verify contract ID matches expectation
   - Don't import unknown consignments

4. **Sync After Import**
   - Always click "Sync Wallet" after import
   - Wait for balance refresh
   - Verify expected changes

---

### For Genesis Sync

5. **Use Same Mnemonic**
   - Critical: Must be exact same 12/24 words
   - Test with one word different = won't work
   - Keep mnemonic secure and backed up

6. **Genesis Files Are Reusable**
   - Same file works for multiple devices
   - Can re-export if needed
   - No expiration or one-time limit

7. **No Hurry**
   - Genesis sync is instant
   - No confirmations needed
   - Can do anytime after issuance

---

### For Token Transfers

8. **Download Consignment Immediately**
   - Don't close modal before downloading
   - File is required for recipient
   - Cannot be regenerated later

9. **Verify Bitcoin TX**
   - Click TX ID link to open explorer
   - Confirm TX is in mempool
   - Watch for confirmation

10. **Wait for Confirmation**
    - Don't consider complete until confirmed
    - 1 confirmation sufficient for most cases
    - Larger amounts: wait for more confirmations

11. **Secure File Transfer**
    - Use encrypted messaging when possible
    - Avoid public channels
    - Verify recipient identity

12. **Monitor Mempool**
    - Watch for TX confirmation
    - Check if fee is sufficient
    - May need to wait longer if low fee

13. **Start with Small Amounts**
    - Test transfer flow first
    - Verify round-trip works
    - Then transfer larger amounts

---

### For Recipients

14. **Generate Invoice Correctly**
    - Specify exact amount needed
    - Copy complete invoice string
    - Don't manually edit invoice

15. **Share Invoice Securely**
    - Send via secure channel
    - Verify recipient identity
    - Consider including amount in message

16. **Import Promptly**
    - Import consignment as soon as received
    - Don't delay unnecessarily
    - Verify while TX is still in mempool

17. **Verify Receipt**
    - Check balance increased correctly
    - Verify new UTXO created
    - Confirm contract ID matches

---

### Security Tips

18. **Never Share Private Keys/Mnemonic**
    - Genesis export ≠ wallet export
    - Consignments are safe to share
    - But mnemonic must stay private

19. **Verify Contract IDs**
    - Compare with known good source
    - Check ticker symbol matches
    - Beware of fake/similar tokens

20. **Keep Software Updated**
    - Use latest wallet version
    - Security patches important
    - Backup before updating

---

## Troubleshooting

### "Asset not showing after genesis import"

**Check:**
1. Did you use same mnemonic?
2. Did you sync wallet after import?
3. Is contract ID correct?

**Solution**: Sync wallet, verify mnemonic

---

### "Transfer stuck in pending"

**Check:**
1. Is Bitcoin TX confirmed?
2. Did recipient import consignment?
3. Network congestion?

**Solution**: Wait for confirmation, check mempool

---

### "Cannot create invoice"

**Check:**
1. Do you have confirmed UTXOs?
2. Is wallet synced?
3. Network connected?

**Solution**: Create UTXO, sync, check network

---

### "Transfer shows confirmed but no balance"

**Check:**
1. Did you import the consignment?
2. Did you sync wallet?
3. Is this the correct wallet?

**Solution**: Import consignment, sync wallet

---

## Next Steps

### Completed Features ✅
- Phase 2: Invoice Generation
- Phase 3: Send Transfer
- Phase 4A: Genesis Export
- Phase 4B: Accept Consignment

### Upcoming Features 🚀
- Phase 5: Frontend Polish
  - Transfer history display
  - Better error messages
  - Loading states
  - Activity feed

- Phase 6: Testing & Documentation
  - End-to-end testing
  - Edge case handling
  - User guide
  - Video tutorials

### Future Enhancements 🔮
- Relay server for automatic consignment sharing
- QR code support for invoices
- Batch transfers
- Multi-signature support
- Hardware wallet integration
- Mobile app

---

## Support

For issues or questions:
1. Check this documentation first
2. Review error messages carefully
3. Test on Signet before mainnet
4. Keep consignment files for troubleshooting

---

**Document Version**: 1.0  
**Last Updated**: October 12, 2025  
**Implementation Status**: Production Ready

