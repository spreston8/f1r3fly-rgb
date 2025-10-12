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

## Issue Assets Feature

Bitlight provides a managed RGB20 asset issuance service with an upfront fee structure and a form-based interface.

### Issuance Fee Structure

Before issuing an asset, users must pay a total fee of **0.00034 BTC**, broken down as:

| Component | Amount | Purpose |
|-----------|--------|---------|
| **Service Fee** | 0.0003 BTC | Bitlight Labs platform fee (anti-spam, registry quality, infrastructure) |
| **UTXO Creation Cost** | 0.00004 BTC | Bitcoin network transaction fee (paid to miners) |
| **Total** | 0.00034 BTC | One-time payment per asset issued |

**Key Points:**
- **Pay once per asset** - Each token issuance requires this fee
- **Multiple issuances allowed** - Can issue multiple assets from same wallet
- **Non-refundable** - Acts as economic spam prevention
- **Service fee rationale:** "To enhance the quality of asset issuance" (Bitlight's anti-spam measure)

**Comparison to alternatives:**
- **Direct RGB issuance:** No service fee (only Bitcoin network fees)
- **Counterparty:** Burns 0.5 XCP per asset (~$5-20)
- **Ethereum ERC-20:** Only gas fees (resulted in massive token spam)

Bitlight's fee is a managed service premium - you're paying for convenience and automatic registry listing.

---

### Asset Issuance Form

After paying the fee, users fill out a form specifying the RGB20 asset parameters:

#### 1. RGB20 Name
```
"2-12 characters for rgb20 name"
```

**Purpose:** Full human-readable name of the asset

**Constraints:**
- Minimum: 2 characters
- Maximum: 12 characters
- Examples: "FireFly Token", "TestCoin", "MyAsset"

**Note:** 12-character limit is restrictive compared to other token standards. Likely an RGB20 schema constraint or Bitlight-specific limitation.

---

#### 2. RGB20 Ticker
```
"2-8 characters for rgb20 ticker"
```

**Purpose:** Short symbol/abbreviation for the asset

**Constraints:**
- Minimum: 2 characters
- Maximum: 8 characters
- Examples: "F1R3FLY", "BTC", "USDT", "TEST"

**Note:** Standard ticker format. 8 characters is generous (most tickers are 3-5 characters).

---

#### 3. Precision
```
"1-10 precision"
```

**Purpose:** Number of decimal places the asset supports

**Constraints:**
- Minimum: 1 decimal place
- Maximum: 10 decimal places

**Common values:**
- **0** = No decimals (whole units only, like NFTs or share counts)
- **2** = Like fiat currencies ($1.23)
- **8** = Like Bitcoin (0.00000001 = 1 satoshi equivalent)
- **18** = Like Ethereum (but RGB caps at 10)

**Example:** If precision = 8 and total supply = 1,000,000:
- Display: 1,000,000.00000000 units
- Smallest transferable unit: 0.00000001

**Important:** Precision is **immutable** after issuance. Choose carefully based on your use case.

---

#### 4. Total Supply
```
"total supply"
```

**Purpose:** The total number of tokens to be created

**Characteristics:**
- **Fixed at issuance** - Immutable in RGB20 standard schema
- **No minting/burning** - Unlike Ethereum ERC-20, RGB20 has fixed supply
- Combined with precision to determine actual total

**Example:** For 1 million divisible tokens with 8 decimals:
- Enter: `1000000` (human-readable amount)
- Actual total considering precision: 1,000,000.00000000

**Note:** The exact input format (whether you enter the raw amount or human-readable amount) depends on Bitlight's implementation. Standard practice is human-readable input with precision applied automatically.

---

#### 5. UTXOS Seal
```
Example: a386a1755c1393fe79dee048c4df02150b28d23b82f8d0d421f3d8c9837ede61:1
```

**Purpose:** The specific UTXO (txid:vout) where the initial asset allocation will be bound

**This is THE MOST CRITICAL field:**

**What it means:**
- This UTXO becomes "occupied" with your newly issued tokens
- The entire supply is initially bound to this single UTXO
- This is your "genesis seal" - the origin point of the asset
- You must own this UTXO (it's from your wallet)
- This UTXO must be unoccupied (no existing RGB assets bound to it)
- The Bitcoin value in this UTXO remains yours (RGB is a layer on top)

**How it's selected:**
- Bitlight automatically suggests an unoccupied UTXO from your wallet
- Likely one you created via "Create UTXO" feature
- Could be any existing unoccupied UTXO with sufficient Bitcoin value

**Why this matters:**
- RGB assets are bound to specific UTXOs (seal-based ownership model)
- This is fundamental to RGB's architecture
- To transfer tokens later, you'll spend this UTXO in an RGB transfer transaction
- The UTXO's Bitcoin value can be as small as dust limit

**RGB Seal Concept:**
- "Seal" = A commitment to a specific UTXO
- RGB state is "sealed" to Bitcoin UTXOs
- Breaking the seal (spending the UTXO) requires creating a new seal (in a transfer)

---

### Complete Issuance Workflow

**Step 1: Preparation**
- Ensure wallet has ≥ 0.00034 BTC for fees
- Ensure at least one unoccupied UTXO exists
- If no unoccupied UTXO, use "Create UTXO" feature first

**Step 2: Fee Payment**
- User initiates "Issue Assets"
- Bitlight displays fee breakdown (0.00034 BTC total)
- User confirms payment

**Step 3: Form Submission**
- **Name:** "FireFly Token" (2-12 chars)
- **Ticker:** "F1R3FLY" (2-8 chars)
- **Precision:** 8 (Bitcoin-like)
- **Total Supply:** 1000000
- **UTXO Seal:** [auto-selected unoccupied UTXO]

**Step 4: Transaction Creation**
Bitlight creates a Bitcoin transaction:
- **Input:** Existing wallet UTXOs
- **Output 1:** 0.0003 BTC → Bitlight service fee address
- **Output 2:** RGB genesis seal UTXO (contains asset allocation)
- **Fee:** ~0.00004 BTC → Bitcoin miners

**Step 5: RGB Contract Generation**
- RGB runtime generates the contract with specified parameters
- Contract is "anchored" to the Bitcoin transaction
- Initial allocation is bound to the selected UTXO seal

**Step 6: Broadcast and Confirmation**
- Transaction is broadcast to Bitcoin network
- Once confirmed (1+ blocks), asset exists on-chain (Bitcoin layer)
- RGB state is stored locally in wallet's stash

**Step 7: Registry Listing**
- Bitlight automatically submits asset to their registry
- Asset becomes discoverable at bitlightlabs.com/asset/[network]
- Includes metadata: name, ticker, supply, precision, contract ID

**Result:**
- Your selected UTXO is now "occupied" with 1,000,000.00000000 F1R3FLY tokens
- Asset is registered and discoverable
- You can now transfer tokens to others via RGB consignments

---

### Key Observations

**1. Mandatory UTXO Binding**
- You cannot issue RGB assets without binding them to a UTXO
- This is fundamental to RGB's seal-based architecture
- The UTXO becomes the "anchor" for asset ownership

**2. Immutable Parameters**
- Name, ticker, precision, and total supply are **permanent**
- No way to change these after issuance (unlike some token standards)
- Choose carefully during issuance

**3. Fixed Supply Model**
- RGB20 standard schema has fixed supply
- No minting or burning functions
- Different from Ethereum ERC-20 (which can be mintable/burnable)

**4. Minimal Metadata**
- Only stores: name, ticker, precision, supply
- No description, website, logo, social links in contract
- Registry may add additional metadata externally

**5. Service Fee as Quality Control**
- 0.0003 BTC (~$30 if BTC = $100k) deters spam
- High enough to be meaningful, low enough for indie projects
- Creates economic barrier to registry pollution

**6. Automatic Registry Integration**
- Unlike direct RGB issuance, Bitlight auto-registers assets
- Provides instant discoverability
- Trade-off: pay fee for convenience

---

### Comparison: Bitlight vs Direct RGB Issuance

| Aspect | Bitlight Wallet | Direct RGB (CLI/Libraries) |
|--------|-----------------|----------------------------|
| **Service Fee** | 0.0003 BTC | None |
| **Network Fee** | ~0.00004 BTC | Same (Bitcoin miners) |
| **Registry Listing** | Automatic | Manual (if desired) |
| **User Experience** | Form-based GUI | Technical (contract files, YAML) |
| **Learning Curve** | Low | High |
| **Flexibility** | Limited to form fields | Full control over contract |
| **Best For** | Non-technical users, quick issuance | Developers, custom schemas |

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

## Bitlight's Design Decisions

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

---

## Asset Registry System

Bitlight Labs maintains a public **Asset Registry** that catalogs RGB assets created using their platform. This registry is separate from the RGB protocol itself and serves as a convenience layer for asset discovery and verification.

**Registry URLs:**
- **Mainnet:** https://bitlightlabs.com/asset/mainnet
- **Testnet:** https://bitlightlabs.com/asset/testnet

---

### What the Registry Stores

For each registered RGB asset, the registry displays:

| Field | Description | Example |
|-------|-------------|---------|
| **Token Name** | Human-readable asset name | "F1R3FLY Test Token" |
| **Ticker** | Short symbol (usually 3-5 chars) | "F1R3FLY" |
| **Total Supply** | Total number of units issued | 1,000,000 |
| **Precision** | Decimal places supported | 8 (like Bitcoin) |
| **Address** | Bitcoin address where asset was issued | tb1q... (Signet/Testnet) |
| **Creation Date** | When the asset was issued | "2024-12-15" |
| **Block Height** | Bitcoin block of issuance transaction | #2,450,123 |
| **Contract ID** | Unique RGB contract identifier | rgb:2dKvN8sP7qM...xL4Z9w |

---

### How Registration Works

**The registration process (inferred from the system):**

1. **User Issues Asset via Bitlight Wallet**
   - User creates RGB asset using Bitlight's interface
   - Bitlight wallet issues the contract on Bitcoin
   - Transaction is broadcast and confirmed

2. **Automatic Registry Submission**
   - Bitlight wallet automatically submits asset metadata to registry
   - Includes: name, ticker, supply, precision, contract ID, etc.
   - This happens behind the scenes (user doesn't manually register)

3. **Registry Database Update**
   - Bitlight's registry server stores the asset information
   - Makes it publicly queryable via web interface
   - Indexed by contract ID, ticker, creation date, etc.

4. **Public Discovery**
   - Anyone can visit the registry website
   - Browse all assets issued through Bitlight
   - Search by ticker, name, or contract ID
   - Click through to see full details

---

### Registry Architecture (Inferred)

```
┌─────────────────────────────────────────────────────────────┐
│                     Bitlight Wallet                          │
│  ┌──────────────────┐         ┌──────────────────┐          │
│  │ User Issues      │         │ RGB Runtime      │          │
│  │ RGB Asset        │────────>│ Creates Contract │          │
│  └──────────────────┘         └─────────┬────────┘          │
│                                          │                   │
│                                          v                   │
│                                ┌──────────────────┐          │
│                                │ Bitcoin TX       │          │
│                                │ (Broadcast)      │          │
│                                └─────────┬────────┘          │
└────────────────────────────────────────┼─────────────────────┘
                                         │
                                         │ (Parallel submission)
                                         │
                                         v
                         ┌────────────────────────────┐
                         │  Bitlight Registry API     │
                         │  (Backend Server)          │
                         │                            │
                         │  POST /api/register-asset  │
                         │  {                         │
                         │    contract_id,            │
                         │    ticker,                 │
                         │    name,                   │
                         │    supply,                 │
                         │    precision,              │
                         │    issuance_txid,          │
                         │    block_height,           │
                         │    network                 │
                         │  }                         │
                         └────────────┬───────────────┘
                                      │
                                      v
                         ┌────────────────────────────┐
                         │  Registry Database         │
                         │  (PostgreSQL/MongoDB?)     │
                         │                            │
                         │  Stores asset metadata     │
                         └────────────┬───────────────┘
                                      │
                                      v
                         ┌────────────────────────────┐
                         │  Public Web Interface      │
                         │  bitlightlabs.com/asset/   │
                         │                            │
                         │  - Browse all assets       │
                         │  - Search by ticker/name   │
                         │  - View asset details      │
                         │  - Verify contract ID      │
                         └────────────────────────────┘
```

---

### Key Characteristics

**1. Centralized**
- Bitlight Labs runs the registry server
- They control what gets listed
- Single source of truth for Bitlight ecosystem

**2. Opt-In by Design**
- Assets are only registered if issued through Bitlight wallet
- Assets issued through other wallets won't appear (unless manually submitted)
- Not a comprehensive registry of ALL RGB assets

**3. Network-Specific**
- Separate registries for Mainnet and Testnet
- Same ticker can exist on both networks (different contract IDs)

**4. Read-Only Public Access**
- Anyone can browse the registry
- No authentication required to view
- Provides transparency for Bitlight-issued assets

**5. Verification Purpose**
- Users can verify asset details before transacting
- Check if a contract ID matches expected ticker
- Confirm total supply and other immutable properties

---

### What the Registry Does NOT Do

❌ **Does not validate asset authenticity** - Anyone could theoretically submit fake data (though Bitlight likely validates issuance transactions)

❌ **Does not prevent duplicate tickers** - Multiple assets can have the same ticker (identified by unique contract IDs)

❌ **Does not store RGB state** - Only stores metadata, not token balances or allocations

❌ **Does not enforce uniqueness** - No global namespace like DNS (contract ID is the only true unique identifier)

❌ **Does not track transfers** - Only records initial issuance, not subsequent transactions

---

### Use Cases

**1. Asset Discovery**
```
User: "I want to see what RGB assets exist on Testnet"
Registry: Shows list of all Bitlight-issued assets
User: Browses, finds interesting assets
```

**2. Verification**
```
User receives RGB asset with contract ID: rgb:abc123...
User: Checks registry to see if contract ID is known
Registry: "Yes, this is F1R3FLY Test Token, issued 2024-12-15"
User: Confirms it's the expected asset
```

**3. Due Diligence**
```
Someone claims to send you "USDT-RGB"
You check registry:
- See total supply
- See issuance date
- See issuer address
Verify it's legitimate before accepting
```

**4. Transparency**
```
Project issues an asset and wants public visibility
Registry automatically lists it
Community can audit:
- Is the supply as claimed?
- When was it created?
- What's the contract ID?
```

---

### Comparison to Other Ecosystems

**Ethereum:**
- **Similar:** CoinGecko, CoinMarketCap (centralized registries)
- **Similar:** Token Lists (JSON files of verified tokens)
- **Different:** Ethereum has on-chain ERC-20 events (more discoverable)

**Bitcoin Ordinals:**
- **Similar:** ord.io, ordinals.com (indexers/registries)
- **Similar:** Centralized services track inscriptions
- **Different:** Ordinal inscriptions are on-chain, RGB is off-chain

**RGB:**
- **Different:** No on-chain events for discovery
- **Different:** Truly client-side validated (no global state)
- **Result:** Registries are essential for UX, not just convenience

---

### Security Considerations

**For Users:**

✅ **Always verify contract ID** - Don't trust ticker alone  
✅ **Cross-reference multiple sources** - Check issuer's website, social media  
✅ **Validate consignments** - RGB protocol validates, registry is just metadata  
✅ **Beware of scams** - Anyone can create "USDT" ticker with different contract ID  

**For Issuers:**

✅ **Publish contract ID widely** - Website, docs, social media  
✅ **Submit to multiple registries** - Don't rely on single source  
✅ **Provide direct contract file** - Let users import directly (trustless)  
✅ **Monitor for impersonators** - Check registry for fake versions of your asset  

---

### Open Questions About Bitlight Registry

1. **Does Bitlight provide a public API?**
   - Can we query registry programmatically?
   - Or only via web interface scraping?

2. **Can external wallets submit assets?**
   - Is there an API endpoint for registration?
   - Or is it only auto-registered from Bitlight wallet?

3. **How does Bitlight verify submissions?**
   - Do they check the Bitcoin transaction?
   - Verify the contract file?
   - Or trust wallet submissions?

4. **Is there a rate limit or authentication?**
   - Can we query freely?
   - Need API key?

5. **Do they offer icon/logo hosting?**
   - Assets have visual branding?
   - Or just text metadata?

---

### General Registry Design Patterns

Common best practices observed in RGB asset registries:

**1. Verification Process**
- Verify Bitcoin transaction exists and is confirmed
- Parse RGB contract file to confirm metadata matches
- Check issuer signature (if applicable)

**2. Duplicate Handling**
- Allow multiple assets with same ticker (real world reality)
- Show warning: "Multiple assets use this ticker"
- Sort by creation date, supply, or popularity

**3. Metadata Storage**
```json
{
  "contract_id": "rgb:2dKvN8sP7qM...xL4Z9w",
  "network": "signet",
  "ticker": "F1R3FLY",
  "name": "F1R3FLY Test Token",
  "description": "Test token for RGB development",
  "precision": 8,
  "total_supply": "1000000",
  "issuance_txid": "abc123...",
  "issuance_vout": 0,
  "block_height": 123456,
  "timestamp": "2024-12-15T10:30:00Z",
  "issuer_address": "tb1q...",
  "icon_url": "https://...",
  "website": "https://f1r3fly.com",
  "social": {
    "twitter": "@f1r3fly",
    "telegram": "..."
  },
  "verified": false,
  "tags": ["test", "utility"]
}
```

**4. API Design**
```
GET  /api/assets                    # List all assets
GET  /api/assets/{contract_id}      # Get specific asset
GET  /api/assets/search?q={ticker}  # Search by ticker
POST /api/assets                    # Submit new asset (authenticated)
GET  /api/assets/{contract_id}/icon # Get asset icon
```

**5. Caching Strategy**
- Cache registry responses (assets are immutable after issuance)
- Refresh every 24 hours or on user request
- Store locally to reduce API calls

---

## RGB Asset Transfer Flow

This section documents the complete RGB asset transfer process as implemented in Bitlight, based on real-world usage and observations.

---

### Overview: Invoice-Based Transfer Model

**Critical Difference from Bitcoin/Ethereum:**

RGB does NOT use addresses for transfers. Instead, it uses **invoices** that encode the recipient's UTXO seal.

| Traditional Model | RGB Model |
|-------------------|-----------|
| "Send to address bc1q..." | "Send to invoice rgb:invoice:..." |
| Address is reusable | Invoice is single-use |
| Sender picks destination | Recipient picks destination (UTXO) |
| Public address | Blinded seal (private) |

---

### Transfer Prerequisites

Before transferring RGB assets, both parties must:

**Sender:**
- ✅ Have RGB assets (occupied UTXO)
- ✅ Have unoccupied UTXO for Bitcoin fees
- ✅ Asset imported in wallet (contract known)

**Recipient:**
- ✅ Asset imported in wallet (contract ID)
- ✅ Have unoccupied UTXO (for receiving)
- ✅ Generate invoice with amount and seal

---

### Step 1: Import Asset (Contract ID)

**Purpose:** Make your wallet aware of a specific RGB asset so you can track, receive, and send it.

**Process:**
1. User obtains contract ID (from issuer, registry, or consignment)
2. User: "Import asset" → Enter contract ID
3. Wallet queries local stash: Already have this contract?
4. If not found: Request contract file (from sender/registry/backup)
5. Wallet validates contract structure and schema
6. Wallet imports into RGB runtime
7. Wallet scans all UTXOs for this asset
8. Displays balance (if any allocations found)

**Result:** Wallet now tracks this asset and can generate invoices for it.

---

### Step 2: Generate Invoice (Recipient)

**Scenario:** Alice wants to send you 100 F1R3FLY tokens

**Your Steps (Recipient):**

```
1. Select asset in wallet
   - Choose: F1R3FLY (from imported contracts)
   
2. Click "Receive" or "Generate Invoice"
   
3. Enter amount
   - Amount: 100 F1R3FLY
   - (Or leave blank for "any amount")
   
4. Wallet selects UTXO seal
   - Automatically picks an unoccupied UTXO
   - Or lets you manually select one
   - This UTXO will receive the tokens
   
5. Wallet generates RGB Invoice
   - Encodes: contract ID, amount, blinded seal, network
   - Format: String like "rgb:invoice:abc123..."
   - Contains payment instructions for sender
   
6. Copy invoice
   - Display as: Text string, QR code, or both
   - Share with sender via any channel (email, chat, etc.)
```

**Invoice Contents:**
```
RGB Invoice:
├─ Contract ID: rgb:2dKvN8sP7qM...xL4Z9w (which asset)
├─ Amount: 100 F1R3FLY (tokens requested)
├─ Beneficiary seal: [BLINDED] (your UTXO, hidden from sender)
├─ Network: signet (prevents wrong-network sends)
└─ Expiration: [optional] (invoice valid until X)
```

**Why Blinded Seal?**
- You selected UTXO: `xyz789...abc:1`
- Invoice contains: `blind(xyz789...abc:1, random_secret)`
- Sender sees: Cryptographic commitment (not your actual UTXO)
- Privacy: Sender can't track your future transactions

---

### Step 3: Send Payment (Sender)

**Scenario:** Alice received your invoice and wants to send 100 F1R3FLY

**Alice's Steps:**

```
1. Open wallet → Select F1R3FLY asset
   
2. Click "Send"
   
3. Paste recipient's invoice
   - Wallet parses invoice automatically
   
4. Review transfer details (Bitlight UI):
   ┌─────────────────────────────────────────┐
   │ Send RGB assets                         │
   │                                         │
   │ Name: F1r3flyTest1                      │
   │ Iface: RGB20                            │
   │ Contract ID: contract:EHK...hXl0zI      │
   │                                         │
   │ Amount:                                 │
   │   Available: 90 FTST1                   │
   │   Sending: 10 FTST1                     │
   │                                         │
   │ Min Locked Amount: 5,000 sats 🔒       │
   │   (Bitcoin value stays with your change)│
   │                                         │
   │ Add UTXO Balance:                       │
   │   Move BTC to pre-fund UTXO for fees   │
   │   [Checks for unoccupied fee UTXO]     │
   │                                         │
   │ Fee: ~200 sats                          │
   │                                         │
   │        [ Approve Payment ]              │
   └─────────────────────────────────────────┘
   
5. Click "Approve Payment"
   
6. Wallet creates RGB state transition:
   - Input: Alice's occupied UTXO (90 FTST1 + 5,000 sats)
   - Output 1: Recipient seal (10 FTST1 + ??? sats)
   - Output 2: Alice's change (80 FTST1 + 5,000 sats)
   - Output 3: Fee UTXO (if needed)
   
7. Wallet creates Bitcoin transaction:
   - Spends Alice's occupied UTXO
   - Creates outputs matching RGB state transition
   - Signs transaction
   
8. Wallet generates Consignment:
   - Complete asset history (back to genesis)
   - State transition proof
   - Bitcoin anchoring commitments
   - Validation data
   
9. Wallet broadcasts Bitcoin TX to network
   
10. Bitlight backend (automatic):
    - Uploads consignment to relay server
    - Notifies recipient's wallet
    - No manual file sharing needed!
```

**What Happens to Sender's Balance:**

```
BEFORE "Approve Payment":
Available: 90 FTST1
Unconfirmed: 0 FTST1

IMMEDIATELY AFTER "Approve Payment":
Available: 0 FTST1        ← Old UTXO spent!
Unconfirmed: 80 FTST1     ← Change UTXO pending

AFTER BITCOIN CONFIRMATION (~10 min):
Available: 80 FTST1       ← Change UTXO confirmed
Unconfirmed: 0 FTST1
```

**Why Balance Shows Zero:**
- Old UTXO (90 FTST1) is SPENT (gone from wallet)
- New change UTXO (80 FTST1) is UNCONFIRMED (not counted yet)
- RGB wallets only count confirmed UTXOs for safety
- This is temporary (resolves after Bitcoin confirmation)

---

### Step 4: Receive & Validate (Recipient)

**Recipient's Experience:**

```
1. Bitlight wallet receives notification
   - Push alert or in-app notification
   - "Incoming transfer: 10 FTST1"
   
2. Wallet auto-downloads consignment
   - From Bitlight relay server
   - No manual import needed
   
3. Wallet validates consignment:
   ✓ Contract ID matches expected asset
   ✓ Amount matches invoice (10 FTST1)
   ✓ All state transitions are valid
   ✓ Complete history back to genesis
   ✓ Bitcoin transaction exists (on-chain or mempool)
   ✓ Beneficiary seal matches invoice
   
4. WHILE BITCOIN TX IS PENDING:
   ┌─────────────────────────────────────────┐
   │ Activity                                │
   │                                         │
   │ 10/11/2025                              │
   │ Receiving                               │
   │ tx: 7a70...b9cb                         │
   │ 10 FTST1                                │
   │ Status: Pending ⏳                      │
   │ [Link to mempool.space]                 │
   └─────────────────────────────────────────┘
   
   Balance: 0 FTST1 (still waiting)
   
5. AFTER BITCOIN TX CONFIRMS:
   ✓ Wallet accepts state transition
   ✓ Updates local RGB stash
   ✓ UTXO from invoice is now "occupied"
   ✓ Balance updated: +10 FTST1
   
   ┌─────────────────────────────────────────┐
   │ Activity                                │
   │                                         │
   │ 10/11/2025                              │
   │ Received ✓                              │
   │ tx: 7a70...b9cb                         │
   │ 10 FTST1                                │
   │ Status: Confirmed                       │
   └─────────────────────────────────────────┘
   
   Balance: 10 FTST1 (available)
```

---

### Complete Transfer Timeline

```
T = 0:00 - Recipient generates invoice
├─ Picks unoccupied UTXO for receiving
├─ Blinds the seal for privacy
└─ Shares invoice with sender

T = 0:30 - Sender pastes invoice
├─ Wallet parses and validates
├─ Shows transfer preview
└─ Sender clicks "Approve"

T = 0:31 - Transaction created
├─ Bitcoin TX: Spends sender's occupied UTXO
├─ RGB consignment: Generated
├─ Bitlight relay: Uploads consignment
└─ Bitcoin network: TX broadcast to mempool

T = 0:32 - Both parties see "Pending"
├─ Sender balance: 0 FTST1 (old spent, new unconfirmed)
├─ Recipient balance: 0 FTST1 (waiting for confirmation)
└─ Bitlight has notified recipient

T = 10:00 - Bitcoin block confirmation
├─ Bitcoin TX: Included in block #2,500,123
├─ Sender change UTXO: Now confirmed (80 FTST1)
├─ Recipient UTXO: Now confirmed + occupied (10 FTST1)
└─ Both wallets update to "Confirmed"

T = 10:01 - Transfer complete
├─ Sender: 80 FTST1 (available for next send)
├─ Recipient: 10 FTST1 (available for next send)
└─ Both can generate new invoices
```

---

### Key Field Explanations

#### "Min Locked Amount" 🔒

**What you see:** Slider (disabled) showing sats amount

**What it means:** The Bitcoin value tied to your RGB asset UTXO

```
Your Occupied UTXO:
┌──────────────────────────────┐
│ Outpoint: abc123...xyz:0     │
│ Bitcoin: 5,000 sats          │ ← Min Locked Amount
│ RGB: 90 FTST1                │
└──────────────────────────────┘

After sending 10 FTST1:
┌──────────────────────────────┐
│ Your Change UTXO (NEW)       │
│ Outpoint: def456...uvw:1     │
│ Bitcoin: 5,000 sats          │ ← PRESERVED
│ RGB: 80 FTST1                │
└──────────────────────────────┘
```

**Why it exists:**
- RGB assets need Bitcoin backing for future transactions
- The sats pay for future transaction fees
- Prevents creating unusable "dust" UTXOs

**Why you can't change it:**
- Source UTXO only has 5,000 sats (can't give more)
- Must maintain minimum viable amount (dust limit)
- Bitlight protects you from creating unspendable UTXOs

---

#### "Add UTXO Balance"

**What you see:** "Move BTC to pre-fund UTXO for RGB20 transaction fees"

**What it means:** Check for unoccupied UTXO to pay Bitcoin network fee

**The problem it solves:**

RGB transfer creates multiple outputs:
1. Recipient UTXO (10 FTST1) - OCCUPIED
2. Your change UTXO (80 FTST1) - OCCUPIED
3. Fee payment UTXO - MUST BE UNOCCUPIED

If all your UTXOs are occupied, you're stuck! Can't pay fees.

**What Bitlight does:**
1. Checks: Do you have unoccupied UTXO with enough BTC?
2. If YES: Uses it for fee payment ✓
3. If NO: Offers to create one (via "Create UTXO" feature)
4. You must have fee UTXO before transfer completes

**Analogy:**
```
RGB tokens: Gift card (occupied UTXO)
Bitcoin for fees: Cash in wallet (unoccupied UTXO)

You need BOTH to complete checkout!
```

---

### Bitlight's Automatic Consignment Handling

**What makes Bitlight special:**

Traditional RGB wallets require **manual consignment sharing**:
```
1. Sender: Generate consignment → Save .rgb file
2. Sender: Email/upload file to recipient
3. Recipient: Download .rgb file
4. Recipient: Import into wallet
5. Recipient: Validate and accept
```

Bitlight automates this via **relay service**:
```
1. Sender: Click "Approve Payment"
2. Bitlight backend: Handles everything
3. Recipient: Notification appears
4. Recipient: See "Pending" automatically
5. Both: No file management needed
```

**Architecture:**
```
┌─────────────────────────────────────────────────────────┐
│              Bitlight Backend Relay Service              │
│                                                          │
│  Sender Wallet ──[Upload Consignment]──> RGB Relay      │
│                                              │           │
│                                              v           │
│                                        [Store + Notify]  │
│                                              │           │
│  Recipient Wallet <──[Auto-download]─────────┘          │
└─────────────────────────────────────────────────────────┘
```

**Trade-offs:**

✅ **Pros:**
- Seamless UX (feels like Venmo/Cash App)
- Real-time notifications
- No file management
- Works great for Bitlight-to-Bitlight

❌ **Cons:**
- Centralized (depends on Bitlight server)
- Privacy reduced (Bitlight sees all transfers)
- Doesn't work cross-wallet (other RGB wallets can't auto-receive)
- Single point of failure (if server down)

**Fallback:**
- If Bitlight server fails, consignment can be regenerated
- Can be shared manually via any channel
- Bitcoin TX is already on-chain (irreversible)
- Pure RGB protocol still works

---

### Blinded Seals (Privacy Feature)

**What you experienced:** Sender doesn't see recipient's address/UTXO

**How it works:**

When recipient creates invoice:
```
1. Wallet selects UTXO: xyz789...abc:1
2. Generates random secret: s = random()
3. Blinds the seal: B = commit(xyz789...abc:1, s)
4. Invoice contains: B (blinded commitment)
5. Recipient stores: (UTXO, s) privately
```

When sender uses invoice:
```
1. Sender wallet sees: B (cannot reverse to actual UTXO)
2. Creates RGB state to: B (blinded recipient)
3. Creates Bitcoin TX with outputs
4. During finalization: Seal is "revealed" (unblinded)
5. But sender UI never shows actual UTXO
```

**Privacy comparison:**

| Method | Sender Knows | Chain Analysis Possible? |
|--------|-------------|-------------------------|
| **Bitcoin** | Recipient address (bc1q...) | ✓ Yes (track address) |
| **Ethereum** | Recipient address (0x...) | ✓ Yes (track address) |
| **RGB (blinded)** | Blinded commitment only | ✗ No (UTXO hidden) |

**Exception:**
- Bitcoin transaction is public (on-chain)
- Anyone can see output addresses
- But can't tell WHICH output holds RGB assets
- RGB state mapping is off-chain (private)

---

### Transfer Requirements Summary

**Before you can send:**
- ✅ Have RGB assets (in occupied UTXO)
- ✅ Have unoccupied UTXO for fees
- ✅ Asset imported in wallet
- ✅ Recipient provides valid invoice

**Before you can receive:**
- ✅ Have unoccupied UTXO (for receiving assets)
- ✅ Asset imported in wallet (know contract ID)
- ✅ Generate invoice with correct amount

**After transfer completes:**
- ✅ Sender: Has change in new UTXO (if any)
- ✅ Recipient: Has assets in UTXO from invoice
- ✅ Both: Bitcoin values preserved (minus network fee)
- ✅ Both: Can immediately transfer again

---

### Activity Tab Display

**Sender's Activity:**
```
┌─────────────────────────────────────────┐
│ F1R3FLYTEST1                            │
│ Available: 80 FTST1                     │
│ Unconfirmed: 0 FTST1                    │
│                                         │
│ Activity:                               │
│ ┌─────────────────────────────────────┐ │
│ │ 10/11/2025                          │ │
│ │ Sent                                │ │
│ │ tx: 7a70...b9cb                     │ │
│ │ 10 FTST1                            │ │
│ │ Status: Confirmed ✓                 │ │
│ └─────────────────────────────────────┘ │
│                                         │
│ ┌─────────────────────────────────────┐ │
│ │ 09/23/2025                          │ │
│ │ Received                            │ │
│ │ tx: 0fb3...4677                     │ │
│ │ 90 FTST1                            │ │
│ │ Status: Confirmed ✓                 │ │
│ └─────────────────────────────────────┘ │
└─────────────────────────────────────────┘
```

**Recipient's Activity:**
```
┌─────────────────────────────────────────┐
│ F1R3FLYTEST1                            │
│ Available: 10 FTST1                     │
│ Unconfirmed: 0 FTST1                    │
│                                         │
│ Activity:                               │
│ ┌─────────────────────────────────────┐ │
│ │ 10/11/2025                          │ │
│ │ Received                            │ │
│ │ tx: 7a70...b9cb                     │ │
│ │ 10 FTST1                            │ │
│ │ Status: Confirmed ✓                 │ │
│ └─────────────────────────────────────┘ │
└─────────────────────────────────────────┘
```

**"Send Details" Modal:**
```
┌─────────────────────────────────────────┐
│ Send Details                            │
│                                         │
│ Transaction amount: 10 FTST1            │
│ Broadcast status: Broadcast Pending     │
│ Recipient's invoice: rgb:invoice:...    │
│ Time: 2025-10-11T22:30:18.755787Z       │
│                                         │
│ Bitcoin TX: 7a70...b9cb                 │
│ [View on Mempool.space]                 │
└─────────────────────────────────────────┘
```

---

### Comparison: Bitlight vs Pure RGB Protocol

| Aspect | Pure RGB (Standard Protocol) | Bitlight Wallet |
|--------|------------------------------|-----------------|
| **Consignment Delivery** | Manual (email/IPFS/any channel) | Automatic (relay server) |
| **User Action** | Export → Share → Import | Just "Approve Payment" |
| **Recipient Notification** | Manual ("Check your email") | Automatic (push alert) |
| **Cross-Wallet** | Universal (works with any RGB wallet) | Optimal for Bitlight-to-Bitlight |
| **Decentralization** | Fully decentralized (no intermediary) | Centralized relay (convenience) |
| **Privacy from Provider** | Full (provider doesn't see transfers) | Reduced (Bitlight sees metadata) |
| **File Management** | Required (.rgb files) | Hidden (automatic) |
| **Learning Curve** | High (understand consignments) | Low (feels like normal app) |
| **Single Point of Failure** | None (peer-to-peer) | Bitlight server (if down) |
| **Fallback Option** | N/A (already manual) | Can export/share consignment manually |

**Bitlight's Design Philosophy:**
- Prioritize UX over pure decentralization
- Similar to Lightning wallets (hide channel management)
- Similar to MetaMask (hide node infrastructure)
- Make RGB usable for mainstream users
- Keep protocol compatibility (can fall back to manual)

---

### Common Transfer Issues & Solutions

#### Issue 1: "Insufficient Funds" Error

**Problem:** Trying to send more tokens than you have

**Solution:**
- Check balance in wallet
- Remember unconfirmed tokens don't count
- Wait for previous transactions to confirm

---

#### Issue 2: "No Fee UTXO Available"

**Problem:** All UTXOs are occupied, can't pay Bitcoin fee

**Solution:**
- Use "Create UTXO" feature
- Create unoccupied UTXO with ~30,000 sats
- Wait for confirmation, then retry transfer

---

#### Issue 3: Balance Shows Zero After Sending

**Problem:** Old UTXO spent, new change UTXO unconfirmed

**Solution:**
- This is normal! Just temporary
- Check "Unconfirmed" field (shows your change)
- Wait ~10 minutes for Bitcoin confirmation
- Balance will update automatically

---

#### Issue 4: "Transfer Pending" for Hours

**Problem:** Bitcoin transaction not confirmed

**Possible causes:**
- Low fee rate (tx stuck in mempool)
- Network congestion (many pending txs)
- Miner's mempool full (tx not propagated)

**Solution:**
- Check mempool.space for tx status
- Wait for next block (usually 10 min average)
- If urgent: Use RBF (Replace-By-Fee) if supported
- If very stuck: Contact Bitlight support

---

#### Issue 5: "Invalid Invoice" Error

**Problem:** Invoice expired, wrong network, or wrong asset

**Solution:**
- Request new invoice from recipient
- Verify network matches (Signet/Testnet/Mainnet)
- Verify contract ID matches asset you want to send
- Check invoice hasn't been used already

---

### Security Best Practices

**For Senders:**
1. ✅ Always verify invoice contract ID matches expected asset
2. ✅ Double-check amount before approving
3. ✅ Use invoices only once (don't reuse old ones)
4. ✅ Keep unoccupied UTXOs for fees
5. ✅ Wait for confirmations before considering transfer "done"

**For Recipients:**
1. ✅ Only share invoices with intended sender
2. ✅ Generate fresh invoice for each receive
3. ✅ Verify consignment before accepting (wallet does this)
4. ✅ Wait for Bitcoin confirmation before trusting balance
5. ✅ Don't reuse UTXOs from invoices for other purposes

**General:**
1. ✅ Contract ID is the source of truth (not ticker)
2. ✅ Always validate consignments (wallet does automatically)
3. ✅ Keep backups of consignment files (if manual sharing)
4. ✅ Understand Bitcoin TX fees affect RGB transfers
5. ✅ Monitor both RGB balance and Bitcoin UTXO state

---

## Summary: Bitlight Wallet Analysis

**Key Takeaways:**

### UTXO Management
1. **Three-Tab System:** Occupied, Unoccupied, and Unlockable categorization provides clear organization
2. **Proactive UTXO Creation:** The "Create UTXO" feature prevents users from being stuck without fee UTXOs
3. **Safe Unlock Mechanism:** Strong warnings and explicit asset forfeit lists protect users from mistakes
4. **Smart Defaults:** 0.0003 BTC UTXO size and 2 sat/vB fee rate are proven sensible defaults

### Registry System
1. **Centralized Convenience Layer:** Bitlight's registry is separate from RGB protocol, providing metadata storage and discovery
2. **Automatic Registration:** Assets issued through Bitlight wallet are automatically submitted to their registry
3. **Public Transparency:** Anyone can browse and verify asset details on testnet and mainnet
4. **Network Isolation:** Separate registries for different Bitcoin networks (prevents confusion)
5. **Metadata-Only:** Registry doesn't store RGB state, just issuance metadata

### Design Principles
1. **Transparency:** Always show which assets are bound, fees, and consequences of actions
2. **Safety:** Explicit warnings for destructive operations (unlocking occupied UTXOs)
3. **Efficiency:** Pre-create resources to avoid workflow bottlenecks
4. **Visual Clarity:** Color-coded badges and clear categorization for quick understanding

### Important Security Notes
- ✅ Registries are convenience layers, not security layers
- ✅ Always validate via RGB consignments for trustless verification
- ✅ Contract ID is the only true unique identifier (tickers can be duplicated)
- ✅ Multiple registries can exist for the same ecosystem (no global namespace)

