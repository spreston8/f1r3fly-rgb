use std::convert::Infallible;
use std::fs;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::OnceLock;

use amplify::ByteArray;
use bitcoin::hashes::Hash;
use bpstd::seals::TxoSeal;
use bpstd::{Outpoint, Vout};
use chrono::Utc;
use commit_verify::{Digest, DigestExt, Sha256};
use hypersonic::StateName;
use rgb::{Assignment, Consensus, Contracts, CreateParams, Issuer};
use rgb_persist_fs::StockpileDir;
use serde::{Deserialize, Serialize};
use strict_encoding::TypeName;

// Embed RGB20 schema at compile time (bundled in binary from wallet/assets/)
const RGB20_ISSUER_BYTES: &[u8] = include_bytes!("../../../assets/RGB20-FNA.issuer");

// Cache issuer (loaded once)
static RGB20_ISSUER: OnceLock<Issuer> = OnceLock::new();

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoundAsset {
    pub asset_id: String,
    pub asset_name: String,
    pub ticker: String,
    pub amount: String,
}

pub struct RgbManager {
    data_dir: PathBuf,
}

impl RgbManager {
    pub fn new(data_dir: PathBuf) -> Result<Self, crate::error::WalletError> {
        // Ensure RGB data directory exists
        fs::create_dir_all(&data_dir).map_err(|e| {
            crate::error::WalletError::Rgb(format!("Failed to create RGB data directory: {}", e))
        })?;

        // Auto-create RGB20 issuer file if not present
        let issuer_path = data_dir.join("RGB20-FNA.issuer");
        if !issuer_path.exists() {
            fs::write(&issuer_path, RGB20_ISSUER_BYTES).map_err(|e| {
                crate::error::WalletError::Rgb(format!("Failed to create RGB20 issuer file: {}", e))
            })?;
        }

        Ok(Self { data_dir })
    }

    fn load_contracts(
        &self,
    ) -> Result<Contracts<StockpileDir<TxoSeal>>, crate::error::WalletError> {
        let stockpile = StockpileDir::load(self.data_dir.clone(), Consensus::Bitcoin, true)
            .map_err(|e| {
                crate::error::WalletError::Rgb(format!("Failed to load RGB stockpile: {}", e))
            })?;

        Ok(Contracts::load(stockpile))
    }

    pub fn check_utxo_occupied(
        &self,
        txid: bitcoin::Txid,
        vout: u32,
    ) -> Result<bool, crate::error::WalletError> {
        let contracts = match self.load_contracts() {
            Ok(c) => c,
            Err(_) => return Ok(false),
        };

        let bp_txid = bpstd::Txid::from_byte_array(txid.to_byte_array());
        let target_outpoint = Outpoint::new(bp_txid, Vout::from_u32(vout));

        for contract_id in contracts.contract_ids() {
            let state = contracts.contract_state(contract_id);

            for (_state_name, owned_states) in state.owned {
                for owned_state in owned_states {
                    // TxoSeal contains an outpoint - extract it for comparison
                    let seal_outpoint = owned_state.assignment.seal.primary;
                    if seal_outpoint == target_outpoint {
                        return Ok(true);
                    }
                }
            }
        }

        Ok(false)
    }

    pub fn get_bound_assets(
        &self,
        txid: bitcoin::Txid,
        vout: u32,
    ) -> Result<Vec<BoundAsset>, crate::error::WalletError> {
        let contracts = self.load_contracts()?;

        let bp_txid = bpstd::Txid::from_byte_array(txid.to_byte_array());
        let target_outpoint = Outpoint::new(bp_txid, Vout::from_u32(vout));

        let mut assets = Vec::new();

        for contract_id in contracts.contract_ids() {
            let state = contracts.contract_state(contract_id);
            let articles = contracts.contract_articles(contract_id);

            for (_state_name, owned_states) in state.owned {
                for owned_state in owned_states {
                    let seal_outpoint = owned_state.assignment.seal.primary;
                    if seal_outpoint == target_outpoint {
                        let ticker = state
                            .immutable
                            .get(&StateName::from_str("ticker").map_err(|e| {
                                crate::error::WalletError::Rgb(format!("Invalid state name: {}", e))
                            })?)
                            .and_then(|states| states.first())
                            .map(|s| s.data.verified.to_string())
                            .unwrap_or_else(|| "N/A".to_string());

                        let asset_name = state
                            .immutable
                            .get(&StateName::from_str("name").map_err(|e| {
                                crate::error::WalletError::Rgb(format!("Invalid state name: {}", e))
                            })?)
                            .and_then(|states| states.first())
                            .map(|s| s.data.verified.to_string())
                            .unwrap_or_else(|| articles.issue().meta.name.to_string());

                        let amount = owned_state.assignment.data.to_string();

                        assets.push(BoundAsset {
                            asset_id: contract_id.to_string(),
                            asset_name,
                            ticker,
                            amount,
                        });
                    }
                }
            }
        }

        Ok(assets)
    }

    fn load_issuer(&self) -> Result<&'static Issuer, crate::error::WalletError> {
        RGB20_ISSUER.get_or_init(|| {
            let issuer_path = self.data_dir.join("RGB20-FNA.issuer");
            Issuer::load(&issuer_path, |_, _, _| -> Result<_, Infallible> { Ok(()) }).expect(
                "Failed to load RGB20 issuer - this should never fail as file is auto-created",
            )
        });
        Ok(RGB20_ISSUER.get().unwrap())
    }

    pub fn issue_rgb20_asset(
        &self,
        request: IssueAssetRequest,
    ) -> Result<IssueAssetResponse, crate::error::WalletError> {
        log::debug!("Loading RGB20 issuer from cached data");
        // 1. Load issuer (cached)
        let issuer = self.load_issuer()?;
        let codex_id = issuer.codex_id();
        log::debug!("RGB20 issuer loaded, codex ID: {}", codex_id);

        log::debug!("Loading contracts stockpile");
        // 2. Load contracts and ensure issuer is imported
        let mut contracts = self.load_contracts()?;

        // Import issuer if not already registered (only happens once)
        if !contracts.has_issuer(codex_id) {
            log::debug!("Importing RGB20 issuer into stockpile (first-time setup)");
            contracts.import_issuer(issuer.clone()).map_err(|e| {
                crate::error::WalletError::Rgb(format!("Failed to import RGB20 issuer: {:?}", e))
            })?;
        } else {
            log::debug!("RGB20 issuer already imported");
        }

        // 3. Parse genesis outpoint
        log::debug!("Parsing genesis UTXO: {}", request.genesis_utxo);
        let outpoint = parse_outpoint(&request.genesis_utxo)?;

        // 4. Create params with TypeName
        log::debug!("Creating contract params for: {} ({})", request.name, request.ticker);
        let type_name = TypeName::try_from(request.name.clone()).map_err(|e| {
            crate::error::WalletError::InvalidInput(format!("Invalid asset name: {:?}", e))
        })?;
        let mut params = CreateParams::new_bitcoin_testnet(codex_id, type_name);

        // 5. Add global state
        params = params
            .with_global_verified("ticker", request.ticker.as_str())
            .with_global_verified("name", request.name.as_str())
            .with_global_verified("precision", map_precision(request.precision))
            .with_global_verified("issued", request.supply);
        log::debug!("Contract params configured - Supply: {}, Precision: {}", 
            request.supply, request.precision);

        // 6. Add owned state (initial allocation)
        params.push_owned_unlocked(
            "balance",
            Assignment::new_internal(outpoint, request.supply),
        );

        // 7. Set timestamp
        params.timestamp = Some(Utc::now());

        // 8. Issue contract
        log::debug!("Issuing contract to stockpile (local operation)");
        let noise_engine = self.create_noise_engine();
        let contract_id = contracts
            .issue(params.transform(noise_engine))
            .map_err(|e| {
                crate::error::WalletError::Rgb(format!("Failed to issue contract: {:?}", e))
            })?;

        log::info!("Contract issued locally to stockpile: {}", contract_id);
        log::debug!("Genesis seal: {}", request.genesis_utxo);

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

    /// Issue RGB20 asset with Firefly/Rholang execution
    /// This creates the asset on both Firefly blockchain and via ALuVM
    pub async fn issue_rgb20_asset_with_firefly(
        &self,
        request: IssueAssetRequest,
        firefly_client: &crate::firefly::FireflyClient,
    ) -> Result<IssueAssetResponseWithFirefly, crate::error::WalletError> {
        log::info!("Starting Firefly/Rholang asset issuance");

        // STEP 1: Generate Rholang contract from request
        let rholang_code = self.generate_rgb20_rholang(&request)?;
        log::debug!("Generated Rholang contract ({} bytes)", rholang_code.len());

        // STEP 2: Deploy to Firefly
        log::info!("Deploying Rholang contract to Firefly");
        let deploy_id = firefly_client.deploy(&rholang_code).await.map_err(|e| {
            crate::error::WalletError::Network(format!("Firefly deploy failed: {}", e))
        })?;
        log::info!("Firefly deploy ID: {}", deploy_id);

        // STEP 3: Propose block to include deploy
        log::debug!("Proposing Firefly block");
        let block_hash = firefly_client.propose().await.map_err(|e| {
            crate::error::WalletError::Network(format!("Firefly propose failed: {}", e))
        })?;
        log::info!("Firefly block hash: {}", block_hash);

        // STEP 4: Wait for deploy to be included
        log::info!("Waiting for deploy to be included in block");
        let _confirmed_block = firefly_client
            .wait_for_deploy(&deploy_id, 10)
            .await
            .map_err(|e| {
                crate::error::WalletError::Network(format!("Failed to confirm deploy: {}", e))
            })?;
        log::info!("Deploy confirmed in Firefly block");

        // STEP 5: Issue RGB contract via ALuVM (legacy path)
        log::info!("Issuing RGB contract via ALuVM");
        let rgb_response = self.issue_rgb20_asset(request.clone())?;
        log::info!("RGB contract issued: {}", rgb_response.contract_id);

        // STEP 6: Return combined response
        // For demo purposes, we prove dual-anchor by returning both Bitcoin and Firefly references
        let firefly_contract_data = serde_json::json!({
            "status": "deployed",
            "message": "RGB20 asset contract deployed to Firefly blockchain",
            "deploy_id": deploy_id,
            "block_hash": block_hash,
            "asset_name": request.name,
            "ticker": request.ticker,
            "supply": request.supply,
            "precision": request.precision,
            "genesis_utxo": request.genesis_utxo,
            "timestamp": chrono::Utc::now().timestamp(),
            "contract_type": "RGB20-FNA"
        });

        Ok(IssueAssetResponseWithFirefly {
            contract_id: rgb_response.contract_id,
            genesis_seal: rgb_response.genesis_seal,
            firefly_deploy_id: deploy_id,
            firefly_block_hash: block_hash,
            firefly_contract_data,
        })
    }

    /// Generate Rholang code for RGB20 asset issuance
    fn generate_rgb20_rholang(
        &self,
        request: &IssueAssetRequest,
    ) -> Result<String, crate::error::WalletError> {
        // Load template
        let template = include_str!("../../../rholang/rgb20_asset.rho");

        let timestamp = chrono::Utc::now().timestamp();

        // Instantiate contract with parameters
        // The contract executes and logs to stdout, proving the deploy succeeded
        let rholang_code = format!(
            r#"
{}

// Instantiate RGB20 asset contract with parameters
new return, stdout(`rho:io:stdout`) in {{
  @"RGB20Asset"!(
    "{}",      // name
    "{}",      // ticker
    {},        // precision
    {},        // supply
    "{}",      // genesis_utxo
    {},        // timestamp
    *return
  ) |
  
  // Log the result for verification
  for(@result <- return) {{
    stdout!(["RGB20 Asset Issuance Complete:", result])
  }}
}}
            "#,
            template,
            request.name,
            request.ticker,
            request.precision,
            request.supply,
            request.genesis_utxo,
            timestamp
        );

        Ok(rholang_code)
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct IssueAssetRequest {
    pub name: String,         // 2-12 chars
    pub ticker: String,       // 2-8 chars
    pub precision: u8,        // 0-10
    pub supply: u64,          // Total supply
    pub genesis_utxo: String, // "txid:vout"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IssueAssetResponse {
    pub contract_id: String,
    pub genesis_seal: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IssueAssetResponseWithFirefly {
    pub contract_id: String,
    pub genesis_seal: String,
    pub firefly_deploy_id: String,
    pub firefly_block_hash: String,
    pub firefly_contract_data: serde_json::Value,
}

// Helper: Parse UTXO outpoint
fn parse_outpoint(utxo_str: &str) -> Result<Outpoint, crate::error::WalletError> {
    let parts: Vec<&str> = utxo_str.split(':').collect();
    if parts.len() != 2 {
        return Err(crate::error::WalletError::InvalidInput(
            "Invalid UTXO format, expected txid:vout".into(),
        ));
    }

    let txid = bpstd::Txid::from_str(parts[0])
        .map_err(|e| crate::error::WalletError::InvalidInput(format!("Invalid txid: {}", e)))?;
    let vout = parts[1]
        .parse::<u32>()
        .map_err(|e| crate::error::WalletError::InvalidInput(format!("Invalid vout: {}", e)))?;

    Ok(Outpoint::new(txid, Vout::from_u32(vout)))
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
