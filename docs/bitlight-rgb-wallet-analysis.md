# Bitlight RGB Wallet - UX Analysis

This document describes the user experience and functionality of the Bitlight RGB wallet extension, specifically focusing on RGB UTXO management.

---

## Overview

Bitlight provides a sophisticated UTXO management system that categorizes Bitcoin outputs based on their RGB asset status. This enables users to:
- Track which UTXOs have RGB assets bound to them
- Pre-create UTXOs for RGB transaction fees
- Unlock/spend UTXOs when needed (with appropriate warnings)

---

## UTXO Categories

The wallet organizes UTXOs into three primary tabs:

### 1. 🔴 **Occupied**

**Definition:** UTXOs that have RGB assets "sealed" to them.

**Example Display:**
```
Output
0fb39b2931...5e4677:1

Available UTXO balance
0.00003692 BTC

Bind RGB assets:
F
F1R3FLYTEST1
contract...l0zI
```

**Characteristics:**
- Contains both Bitcoin value AND RGB token allocations
- Cannot spend the Bitcoin without also handling the RGB asset
- Shows which RGB assets are bound to the UTXO
- Contract ID is displayed (truncated)

**Use Case:** These UTXOs are "in use" by RGB tokens and should generally not be spent unless transferring the tokens.

---

### 2. 🟢 **Unoccupied**

**Definition:** Regular Bitcoin UTXOs with no RGB assets attached.

**Example Display:**
```
Output
b37116ced5...1f19b1:0

Available UTXO balance
0.0003 BTC

---

Output
fff580e388...94cece:0

Available UTXO balance
0.000021 BTC
```

**Characteristics:**
- Contains only Bitcoin value
- Free to spend at any time
- Can be used as recipients for RGB transfers
- Can be used to pay transaction fees for RGB operations

**Use Case:** These are available for regular Bitcoin transactions or as change/fee UTXOs for RGB operations.

---

### 3. 🔓 **Unlockable**

**Definition:** ALL UTXOs (both occupied and unoccupied) that can be spent if needed.

**Example Display:**
```
Output
0fb39b2931...5e4677:1

Available UTXO balance
0.00003692 BTC

Bind RGB assets:
F
F1R3FLYTEST1
contract...l0zI

---

Output
b37116ced5...1f19b1:0

Available UTXO balance
0.0003 BTC

---

Output
fff580e388...94cece:0

Available UTXO balance
0.000021 BTC
```

**Characteristics:**
- Shows the union of occupied + unoccupied UTXOs
- Each UTXO can be "unlocked" (spent) individually
- Different unlock behavior depending on whether the UTXO is occupied or unoccupied

**Use Case:** Emergency access to all funds, or consolidation of UTXOs.

---

## Unlock UTXO Feature

The "Unlock" action allows users to spend a UTXO and transfer its Bitcoin value back to the wallet balance.

### For Unoccupied UTXOs

**Notice:**
```
Notice
UTXO unlocking requires a transaction fee. After unlocking, the available 
BTC in the original UTXO will be transferred to your BTC balance.
```

**Behavior:**
- Creates a transaction spending the UTXO
- Sends Bitcoin to a new address in the user's wallet
- User pays network transaction fee
- No RGB asset complications

**Use Case:** Consolidate small UTXOs or recover Bitcoin from unused outputs.

---

### For Occupied UTXOs

**Notice:**
```
Notice
UTXO unlocking requires a transaction fee. After unlocking, the available 
BTC in the original UTXO will be transferred to your BTC balance.

You will forfeit these RGB assets:
F
F1R3FLYTEST1
90FTST1
```

**Behavior:**
- Creates a transaction spending the UTXO
- Sends Bitcoin to a new address in the user's wallet
- **⚠️ WARNING:** RGB assets bound to this UTXO will be FORFEITED (burned)
- User explicitly sees which assets will be lost
- User pays network transaction fee

**Use Case:** Emergency recovery of Bitcoin when RGB assets are no longer valuable, or user error (last resort only).

**Critical Warning:** This destroys RGB assets! Should only be used when:
- The RGB assets have no value
- User made a mistake and wants to recover BTC
- Emergency situation where BTC is more important than tokens

---

## Create UTXO Feature

A key workflow optimization that allows users to pre-create UTXOs specifically for RGB operations.

### Why Create UTXOs?

**Problem:** RGB transfers require:
1. **Change UTXO:** To receive remaining RGB tokens after a transfer
2. **Fee UTXO:** An unoccupied UTXO to pay Bitcoin transaction fees

If all UTXOs are occupied (holding RGB assets), you cannot pay transaction fees!

**Solution:** Pre-create small unoccupied UTXOs dedicated to paying fees.

---

### Mode 1: Default

**Display:**
```
Create RGB UTXO

Default | Custom

Move BTC to pre-fund UTXO for RGB20 transaction fees.

The UTXO creation amount
0.0003 BTC

Balances:
0.00037098 BTC

Fee
2 sat/VB
```

**Characteristics:**
- **Amount:** Fixed at 0.0003 BTC (standard, proven amount)
- **Fee Rate:** 2 sat/vB (displayed, likely adjustable)
- **Purpose:** Quick creation with sensible defaults
- Shows current wallet balance for reference

**Use Case:** Most users should use this - it's the recommended amount for RGB fee UTXOs.

---

### Mode 2: Custom

**Display:**
```
Create RGB UTXO

Default | Custom

Move BTC to pre-fund UTXO for RGB20 transaction fees.

Available BTC
Balances:
0.00037098 BTC

BTC
[input field]

Fee
[input field]
```

**Characteristics:**
- **Amount:** User-specified
- **Fee Rate:** User-specified
- Shows available balance
- Advanced control for power users

**Use Cases:**
- Create larger UTXOs (e.g., 0.001 BTC) for multiple operations
- Create smaller UTXOs (e.g., 0.0001 BTC) to conserve funds
- Adjust fee rate for urgent vs. economical transactions

---

## Implementation Behind the Scenes

### UTXO State Detection

**How does Bitlight determine if a UTXO is "occupied"?**

The wallet likely:
1. Queries the RGB runtime/stash for all known contracts
2. For each UTXO, checks if it appears in any RGB state as:
   - An allocation (holds tokens)
   - A seal (anchor point for RGB data)
3. If found, marks UTXO as "occupied" and stores the associated asset IDs
4. Otherwise, marks as "unoccupied"

### Create UTXO Transaction

**What happens when you create a UTXO?**

1. Select an existing unoccupied UTXO as input (or multiple to reach the amount)
2. Create a transaction with two outputs:
   - **Output 1:** The desired amount (e.g., 0.0003 BTC) to a new address in your wallet
   - **Output 2:** Change back to your wallet (if any)
3. Sign and broadcast the transaction
4. The new UTXO becomes available once confirmed

**Result:** You now have a fresh, unoccupied UTXO ready to use for RGB fees.

### Unlock UTXO Transaction

**What happens when you unlock a UTXO?**

1. Create a transaction spending the target UTXO
2. Send the entire amount (minus fee) to a new address in your wallet
3. If occupied:
   - RGB runtime does NOT create a transfer
   - The RGB assets are effectively burned/lost
   - Wallet warns user explicitly
4. Sign and broadcast
5. Bitcoin is recovered to your wallet

---

## Key UX Principles

### 1. **Transparency**
- Always show which assets are bound to UTXOs
- Clear warnings when actions will forfeit RGB assets
- Display transaction fees upfront

### 2. **Safety**
- Explicit warnings for destructive actions (unlocking occupied UTXOs)
- List exactly which assets will be forfeited
- User must acknowledge before proceeding

### 3. **Efficiency**
- Pre-create UTXOs to avoid being "stuck" without fee UTXOs
- Provide sensible defaults (0.0003 BTC, 2 sat/vB)
- Allow advanced users to customize

### 4. **Categorization**
- Clear separation of occupied vs. unoccupied
- "Unlockable" view shows everything for advanced users
- Visual differentiation (likely colors/icons)

---

## Recommended Amounts

Based on Bitlight's defaults:

- **RGB Fee UTXO:** 0.0003 BTC (~30,000 sats)
  - Sufficient for multiple RGB transactions
  - Not too large to tie up funds unnecessarily
  
- **Transaction Fee Rate:** 2 sat/vB
  - Conservative rate for non-urgent transactions
  - Users can increase for faster confirmation

---

## Workflow Examples

### Example 1: Prepare for RGB Transfer

**Goal:** Transfer 100 RGB tokens to someone

**Steps:**
1. Check "Unoccupied" tab - do you have at least one unoccupied UTXO?
2. If not, use "Create UTXO" with default settings (0.0003 BTC)
3. Wait for confirmation
4. Now you can perform RGB transfer (the unoccupied UTXO pays fees)

---

### Example 2: Issue New RGB Asset

**Goal:** Issue 1000 tokens of a new asset

**Steps:**
1. Check "Unoccupied" tab - ensure you have UTXOs for:
   - Fee payment
   - Initial allocation (will become occupied)
2. If insufficient, create 2 UTXOs:
   - One for fees (0.0003 BTC)
   - One for initial allocation (can be smaller, e.g., 0.0001 BTC)
3. Issue the asset (allocation UTXO becomes occupied)

---

### Example 3: Emergency BTC Recovery

**Goal:** Recover Bitcoin from an occupied UTXO (RGB tokens are worthless)

**Steps:**
1. Go to "Unlockable" tab
2. Find the occupied UTXO with valuable BTC but worthless RGB assets
3. Click "Unlock"
4. **Read the warning carefully** - lists which assets will be forfeited
5. Confirm and proceed
6. Bitcoin is recovered, RGB assets are burned

---

## Design Considerations for Implementation

### Visual Indicators

**Occupied UTXOs:**
- 🔴 Red badge/icon
- Show bound assets prominently
- Contract ID (truncated with tooltip for full ID)

**Unoccupied UTXOs:**
- 🟢 Green badge/icon
- Simple display (just txid:vout and amount)

### Color Scheme

- **Occupied:** Red/orange tones (caution)
- **Unoccupied:** Green tones (available)
- **Unlock button:** Yellow/orange (warning action)

### Modal Dialogs

**Create UTXO Modal:**
- Tabs for Default/Custom
- Clear explanation of purpose
- Show current balance
- Preview transaction (amount + fee)

**Unlock UTXO Modal:**
- Large warning section
- If occupied: List all assets that will be forfeited in red
- Checkbox: "I understand I will lose these RGB assets"
- Confirm button only enabled after checkbox

---

## Technical Requirements

To implement this in our wallet:

### Backend Requirements

1. **UTXO Classification:**
   - Query RGB runtime to determine if UTXO has assets
   - Return UTXO list with `is_occupied` flag and `bound_assets[]`

2. **Create UTXO Transaction:**
   - Build transaction sending BTC to self
   - Sign and broadcast
   - Return new UTXO details

3. **Unlock UTXO Transaction:**
   - Build transaction spending specific UTXO
   - Send to new wallet address
   - Sign and broadcast

### Frontend Requirements

1. **UTXO List with Tabs:**
   - Filter by occupied/unoccupied/all
   - Display bound assets for occupied UTXOs
   - Action buttons per UTXO

2. **Create UTXO Modal:**
   - Default/Custom mode toggle
   - Amount input (custom mode)
   - Fee rate input
   - Balance display

3. **Unlock UTXO Modal:**
   - Warning display
   - Conditional warning (occupied vs. unoccupied)
   - Asset forfeit list
   - Confirmation checkbox/button

---

## Questions for Further Research

1. **Minimum UTXO Size:** Is there a minimum BTC amount for RGB operations?
2. **Dust Limits:** What's the smallest unoccupied UTXO that's useful?
3. **Fee Estimation:** How does Bitlight estimate fees for RGB vs. Bitcoin transactions?
4. **UTXO Selection:** When multiple unoccupied UTXOs exist, which does RGB runtime use for fees?
5. **Batch Operations:** Can you create multiple UTXOs in one transaction?

---

## Summary

Bitlight's RGB UTXO management provides:

✅ Clear categorization (Occupied/Unoccupied/Unlockable)  
✅ Proactive UTXO creation to avoid being "stuck"  
✅ Safe unlock mechanism with clear warnings  
✅ Visual indicators for UTXO states  
✅ Sensible defaults with advanced customization  

This design prioritizes **safety** (warnings), **transparency** (show all details), and **efficiency** (pre-create UTXOs).

