# 🎯 RGB20 Asset Issuance Implementation Plan

## Research Summary

**Status**: ✅ Research Complete  
**Confidence Level**: **9/10** (Very High)  
**Documentation**: See `rgb-runtime-research-findings.md` for detailed research  
**Date**: October 10, 2025

### Key Findings:

1. ✅ **Native API is simpler than expected** - No manual Bitcoin transaction building needed
2. ✅ **RGB20 schema can be embedded** - Use `include_bytes!()` for self-contained binary
3. ✅ **Auto-initialization possible** - Schema file created automatically on first use
4. ✅ **Reference implementation available** - `rgb-wallet` shows working CLI approach
5. ✅ **Clear data structures** - `CreateParams`, `Assignment`, `Issuer`
6. ✅ **Precision values are strings** - "centiMilli", not enums

---

## Implementation Plan (Backend → Frontend)

### 📦 Phase 1: Backend Implementation

#### Step 1.1: Embed and Auto-Initialize RGB20 Schema (2-3 hours)

**Approach: Embed Schema in Binary**

We use `include_bytes!()` to embed the RGB20-FNA.issuer file directly in the compiled binary. This provides:
- ✅ **Self-contained** - No external file dependencies at runtime
- ✅ **Automatic setup** - File is created on first `RgbManager::new()`
- ✅ **Production-ready** - Works in any deployment environment
- ✅ **No manual steps** - Completely transparent to the user

**Schema Location**: `/wallet/assets/RGB20-FNA.issuer` (copied from `/rgb/examples/`)

**File: `wallet/src/wallet/rgb.rs`**

Add at the top of the file (embed schema in binary):
```rust
// Add to dependencies
use rgb::{Issuer, CreateParams, Assignment};
use std::convert::Infallible;
use std::sync::OnceLock;
use bpstd::Outpoint;
use chrono::Utc;
use commit_verify::{Digest, DigestExt, Sha256};

// Embed RGB20 schema at compile time (bundled in binary from wallet/assets/)
const RGB20_ISSUER_BYTES: &[u8] = include_bytes!("../../assets/RGB20-FNA.issuer");

// Cache issuer (loaded once)
static RGB20_ISSUER: OnceLock<Issuer> = OnceLock::new();
```

**Update `RgbManager::new()` to auto-create issuer file:**
```rust
impl RgbManager {
    pub fn new(data_dir: PathBuf, network: bpstd::Network) -> Result<Self, crate::error::WalletError> {
        // Ensure RGB data directory exists
        fs::create_dir_all(&data_dir)
            .map_err(|e| crate::error::WalletError::Io(e))?;
        
        // Auto-create RGB20 issuer file if not present
        let issuer_path = data_dir.join("RGB20-FNA.issuer");
        if !issuer_path.exists() {
            fs::write(&issuer_path, RGB20_ISSUER_BYTES)
                .map_err(|e| crate::error::WalletError::Rgb(
                    format!("Failed to create RGB20 issuer file: {}", e)
                ))?;
        }
        
        Ok(Self { data_dir, network })
    }
    
    fn load_issuer(&self) -> Result<&'static Issuer, crate::error::WalletError> {
        RGB20_ISSUER.get_or_try_init(|| {
            let issuer_path = self.data_dir.join("RGB20-FNA.issuer");
            Issuer::load(&issuer_path, |_, _, _| -> Result<_, Infallible> { Ok(()) })
                .map_err(|e| crate::error::WalletError::Rgb(format!("Failed to load RGB20 issuer: {}", e)))
        })
    }

    pub fn issue_rgb20_asset(
        &self,
        request: IssueAssetRequest,
    ) -> Result<IssueAssetResponse, WalletError> {
        // 1. Load issuer (cached)
        let issuer = self.load_issuer()?;
        
        // 2. Parse genesis outpoint
        let outpoint = parse_outpoint(&request.genesis_utxo)?;
        
        // 3. Create params
        let mut params = CreateParams::new_bitcoin_testnet(
            issuer.codex_id(),
            &request.name
        );
        
        // 4. Add global state
        params = params
            .with_global_verified("ticker", request.ticker.as_str())
            .with_global_verified("name", request.name.as_str())
            .with_global_verified("precision", map_precision(request.precision))
            .with_global_verified("issued", request.supply);
        
        // 5. Add owned state (initial allocation)
        params.push_owned_unlocked(
            "balance",
            Assignment::new_internal(outpoint, request.supply)
        );
        
        // 6. Set timestamp
        params.timestamp = Some(Utc::now());
        
        // 7. Load contracts
        let mut contracts = self.load_contracts()?;
        
        // 8. Issue contract
        let noise_engine = self.create_noise_engine();
        let contract_id = contracts.issue(params.transform(noise_engine))
            .map_err(|e| WalletError::Rgb(format!("Failed to issue contract: {:?}", e)))?;
        
        Ok(IssueAssetResponse {
            contract_id: contract_id.to_string(),
            genesis_seal: request.genesis_utxo,
        })
    }
    
    fn create_noise_engine(&self) -> Sha256 {
        let mut noise = Sha256::new();
        noise.input_raw(b"wallet_noise");
        noise
    }
}

// Helper: Parse UTXO outpoint
fn parse_outpoint(utxo_str: &str) -> Result<Outpoint, WalletError> {
    let parts: Vec<&str> = utxo_str.split(':').collect();
    if parts.len() != 2 {
        return Err(WalletError::InvalidInput("Invalid UTXO format, expected txid:vout".into()));
    }
    
    let txid = bpstd::Txid::from_str(parts[0])
        .map_err(|e| WalletError::InvalidInput(format!("Invalid txid: {}", e)))?;
    let vout = parts[1].parse::<u32>()
        .map_err(|e| WalletError::InvalidInput(format!("Invalid vout: {}", e)))?;
    
    Ok(Outpoint::new(txid, bpstd::Vout::from_u32(vout)))
}

// Helper: Map precision number to string
fn map_precision(precision: u8) -> &'static str {
    match precision {
        0 => "indivisible",
        1 => "deci",
        2 => "centi",
        3 => "milli",
        4 => "deciMilli",
        5 => "centiMilli",
        6 => "micro",
        7 => "deciMicro",
        8 => "centiMicro",
        9 => "nano",
        10 => "deciNano",
        _ => "centiMicro", // Default to 8 decimals
    }
}
```

**Add issuance method to `RgbManager`:**
```rust
    pub fn issue_rgb20_asset(
        &self,
        request: IssueAssetRequest,
    ) -> Result<IssueAssetResponse, crate::error::WalletError> {
        // ... (implementation as shown in original Step 1.2)
    }
    
    fn create_noise_engine(&self) -> Sha256 {
        let mut noise = Sha256::new();
        noise.input_raw(b"wallet_noise");
        noise
    }
}

// Helper: Parse UTXO outpoint
fn parse_outpoint(utxo_str: &str) -> Result<Outpoint, crate::error::WalletError> {
    // ... (implementation as shown in original Step 1.2)
}

// Helper: Map precision number to string
fn map_precision(precision: u8) -> &'static str {
    // ... (implementation as shown in original Step 1.2)
}
```

#### Step 1.2: Add Request/Response Types (15 mins)

**File: `wallet/src/wallet/manager.rs`**

```rust
#[derive(Debug, Clone)]
pub struct IssueAssetRequest {
    pub name: String,           // 2-12 chars
    pub ticker: String,         // 2-8 chars
    pub precision: u8,          // 0-10
    pub supply: u64,            // Total supply
    pub genesis_utxo: String,   // "txid:vout"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IssueAssetResponse {
    pub contract_id: String,
    pub genesis_seal: String,
}
```

#### Step 1.3: Add API Endpoint (30 mins)

**File: `wallet/src/api/handlers.rs`**

```rust
pub async fn issue_asset_handler(
    State(manager): State<Arc<WalletManager>>,
    Path(name): Path<String>,
    Json(req): Json<IssueAssetRequest>,
) -> Result<Json<IssueAssetResponse>, WalletError> {
    // Validate wallet exists
    if !manager.storage.wallet_exists(&name)? {
        return Err(WalletError::WalletNotFound(name));
    }
    
    // Issue asset via RGB manager
    let result = manager.rgb_manager.issue_rgb20_asset(req)?;
    
    Ok(Json(result))
}
```

**File: `wallet/src/api/server.rs`**

```rust
.route("/api/wallet/:name/issue-asset", post(handlers::issue_asset_handler))
```

#### Step 1.4: Verify Dependencies (5 mins)

**File: `wallet/Cargo.toml`**

```toml
# Already added in Phase 3, but verify:
chrono = "0.4"
commit-verify = "0.12"
```

---

### 🎨 Phase 2: Frontend Implementation

#### Step 2.1: Add TypeScript Types (10 mins)

**File: `wallet-frontend/src/api/types.ts`**

```typescript
export interface IssueAssetRequest {
  name: string;           // 2-12 chars
  ticker: string;         // 2-8 chars
  precision: number;      // 0-10
  supply: number;         // Total supply
  genesis_utxo: string;   // "txid:vout"
}

export interface IssueAssetResponse {
  contract_id: string;
  genesis_seal: string;
}

// Add precision options
export const PRECISION_OPTIONS = [
  { value: 0, label: 'Indivisible (0 decimals)', example: '1' },
  { value: 2, label: 'Centi (2 decimals)', example: '0.01' },
  { value: 8, label: 'CentiMicro (8 decimals - Like BTC)', example: '0.00000001' },
];
```

#### Step 2.2: Add API Function (10 mins)

**File: `wallet-frontend/src/api/wallet.ts`**

```typescript
issueAsset: async (
  name: string,
  request: IssueAssetRequest
): Promise<IssueAssetResponse> => {
  const response = await apiClient.post<IssueAssetResponse>(
    `/wallet/${name}/issue-asset`,
    request
  );
  return response.data;
}
```

#### Step 2.3: Create Issue Asset Modal (2-3 hours)

**File: `wallet-frontend/src/components/IssueAssetModal.tsx`**

Structure:
- Form fields: name, ticker, precision (dropdown), supply, genesis UTXO (dropdown from unoccupied)
- Validation: name 2-12 chars, ticker 2-8 chars uppercase, supply > 0
- Success state: Show contract ID with copy button
- Error handling

#### Step 2.4: Integrate into WalletDetail (30 mins)

**File: `wallet-frontend/src/pages/WalletDetail.tsx`**

Add button near "Create UTXO":
```tsx
<button onClick={() => setShowIssueAssetModal(true)}>
  🪙 Issue Asset
</button>

<IssueAssetModal
  walletName={name || ''}
  unoccupiedUtxos={balance?.utxos.filter(u => !u.is_occupied) || []}
  isOpen={showIssueAssetModal}
  onClose={() => setShowIssueAssetModal(false)}
  onSuccess={loadWalletData}
/>
```

---

## Testing Plan

### 1. Backend Testing (curl)
```bash
# Ensure wallet has unoccupied UTXO first
curl -X POST http://localhost:3000/api/wallet/test/issue-asset \
  -H "Content-Type: application/json" \
  -d '{
    "name": "Test Token",
    "ticker": "TEST",
    "precision": 8,
    "supply": 1000000,
    "genesis_utxo": "txid:0"
  }'
```

### 2. Frontend Testing
- Create/import wallet
- Fund wallet with Signet faucet
- Create UTXO (0.0003 BTC)
- Click "Issue Asset"
- Fill form, submit
- Verify contract ID returned
- Refresh balance → UTXO shows as "Occupied" with bound asset

---

## Time Estimates

| Phase | Task | Estimated Time |
|-------|------|----------------|
| **Backend** | Embed schema + Extend RgbManager | 2-3 hours |
| | Add types | 15 mins |
| | Add API endpoint | 30 mins |
| | Testing + fixes | 1-2 hours |
| **Frontend** | Types + API | 20 mins |
| | Issue Asset Modal | 2-3 hours |
| | Integration | 30 mins |
| | Testing | 1 hour |
| **Total** | | **8-11 hours** |

---

## Confidence Assessment

### High Confidence (9/10) ✅

**Why:**
- ✅ Native API is well-documented
- ✅ Reference implementation exists
- ✅ Schema file already available
- ✅ Similar to Phase 3 (same `Contracts` struct)
- ✅ Clear data structures

### Minor Risks (1/10) ⚠️

- `noise_engine` - Need to create properly (simple Sha256 hash)
- `Assignment::new_internal` - Verify this is correct method
- Precision string mapping - Need to test all values

**None of these are blockers** - all have clear solutions from research.

---

## Success Criteria

✅ Issue RGB20 token via form  
✅ Contract ID returned  
✅ Genesis UTXO becomes "Occupied"  
✅ Asset displays in UTXO tabs with ticker/name/amount  
✅ Can issue multiple assets to different UTXOs  
✅ Error handling for invalid inputs  

---

## Implementation Checklist

### Backend
- [x] Copy RGB20-FNA.issuer to `wallet/assets/` (✅ DONE)
- [ ] Add `const RGB20_ISSUER_BYTES: &[u8] = include_bytes!("../../assets/RGB20-FNA.issuer")`
- [ ] Add `OnceLock` for issuer caching
- [ ] Update `RgbManager::new()` to auto-write issuer file from embedded bytes
- [ ] Implement `load_issuer()`
- [ ] Implement `issue_rgb20_asset()`
- [ ] Add `parse_outpoint()` helper
- [ ] Add `map_precision()` helper
- [ ] Add `create_noise_engine()` method
- [ ] Add `IssueAssetRequest` struct
- [ ] Add `IssueAssetResponse` struct
- [ ] Add `issue_asset_handler()`
- [ ] Add route to server
- [ ] Test with curl
- [ ] Ensure compilation

### Frontend
- [ ] Add `IssueAssetRequest` interface
- [ ] Add `IssueAssetResponse` interface
- [ ] Add `PRECISION_OPTIONS` constant
- [ ] Add `issueAsset()` API function
- [ ] Create `IssueAssetModal.tsx`
- [ ] Add form fields (name, ticker, precision, supply, genesis_utxo)
- [ ] Add form validation
- [ ] Add success state display
- [ ] Add error handling
- [ ] Integrate button in `WalletDetail.tsx`
- [ ] Test end-to-end
- [ ] Ensure compilation

---

## Next Steps After Approval

1. Embed RGB20 schema in binary and update `RgbManager::new()`
2. Implement backend (`RgbManager::issue_rgb20_asset`)
3. Test with curl
4. Implement frontend modal
5. End-to-end testing
6. Celebrate! 🎉

---

## References

- **Research Document**: `rgb-runtime-research-findings.md`
- **Phase 3 Plan**: `phase-3-rgb-integration-plan.md`
- **Bitlight Analysis**: `bitlight-rgb-wallet-analysis.md`
- **RGB20 Schema**: `wallet/assets/RGB20-FNA.issuer` (local copy)
- **RGB20 Schema (original)**: `rgb/examples/RGB20-FNA.issuer`
- **Reference Implementation**: `rgb-wallet/backend/src/wallet/rgb.rs`

---

**Status**: ⏳ Awaiting approval to proceed with implementation

