// Bitcoin Validator Module
// Validates F1r3fly state against Bitcoin blockchain using existing Esplora integration

use crate::config::WalletConfig;
use crate::firefly::types::Allocation;
use super::balance::BalanceChecker;
use serde::{Deserialize, Serialize};
use std::fmt;

// ============================================================================
// Bitcoin Transaction Types (Esplora API Response)
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BitcoinTransaction {
    pub txid: String,
    pub version: i32,
    pub locktime: u32,
    pub vin: Vec<TxInput>,
    pub vout: Vec<TxOutput>,
    pub size: u32,
    pub weight: u32,
    pub fee: Option<u64>,
    pub status: TxStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TxInput {
    pub txid: String,
    pub vout: u32,
    pub prevout: Option<TxOutput>,
    pub scriptsig: String,
    pub scriptsig_asm: String,
    pub witness: Option<Vec<String>>,
    pub is_coinbase: bool,
    pub sequence: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TxOutput {
    pub value: u64,
    pub n: u32,
    pub scriptpubkey: ScriptPubKey,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScriptPubKey {
    pub asm: String,
    pub hex: String,
    #[serde(rename = "type")]
    pub script_type: String,
    pub address: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TxStatus {
    pub confirmed: bool,
    pub block_height: Option<u32>,
    pub block_hash: Option<String>,
    pub block_time: Option<u64>,
}

// ============================================================================
// Validation Error Types
// ============================================================================

#[derive(Debug)]
pub enum ValidationError {
    Network(String),
    Parse(String),
    NotFound(String),
    OutputMismatch,
    InputMismatch,
    InsufficientConfirmations,
    InvalidUtxo,
    InvalidAmount,
}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ValidationError::Network(msg) => write!(f, "Network error: {}", msg),
            ValidationError::Parse(msg) => write!(f, "Parse error: {}", msg),
            ValidationError::NotFound(msg) => write!(f, "Not found: {}", msg),
            ValidationError::OutputMismatch => write!(f, "Output mismatch with Bitcoin"),
            ValidationError::InputMismatch => write!(f, "Input mismatch with Bitcoin"),
            ValidationError::InsufficientConfirmations => write!(f, "Insufficient confirmations"),
            ValidationError::InvalidUtxo => write!(f, "Invalid UTXO format"),
            ValidationError::InvalidAmount => write!(f, "Invalid amount"),
        }
    }
}

impl std::error::Error for ValidationError {}

// ============================================================================
// Bitcoin Validator Implementation
// ============================================================================

pub struct BitcoinValidator {
    esplora_url: String,
    min_confirmations: u32,
}

impl BitcoinValidator {
    /// Create a new BitcoinValidator with configuration
    /// Reuses existing BalanceChecker infrastructure
    pub fn new(config: &WalletConfig) -> Self {
        Self {
            esplora_url: config.esplora_url.clone(),
            min_confirmations: 6, // Default to 6 confirmations for security
        }
    }

    /// Create a new BitcoinValidator with custom confirmation requirements
    pub fn new_with_confirmations(config: &WalletConfig, min_confirmations: u32) -> Self {
        Self {
            esplora_url: config.esplora_url.clone(),
            min_confirmations,
        }
    }

    /// Validate F1r3fly allocation against Bitcoin blockchain
    /// This is the core validation method that ensures F1r3fly state matches Bitcoin reality
    pub async fn validate_allocation(
        &self,
        allocation: &Allocation,
    ) -> Result<bool, ValidationError> {
        log::debug!("Validating allocation: UTXO={}, Amount={}", 
                   allocation.bitcoin_txid, allocation.amount);

        // 1. Parse UTXO from bitcoin_txid (assuming format: "txid:vout")
        let (txid, vout) = self.parse_utxo(&allocation.bitcoin_txid)?;

        // 2. Fetch Bitcoin transaction using existing BalanceChecker infrastructure
        let bitcoin_tx = self.get_transaction(&txid).await?;

        // 3. Verify output exists and matches
        self.verify_output(&bitcoin_tx, vout, allocation.amount)?;

        // 4. Check confirmations
        self.verify_confirmations(&bitcoin_tx).await?;

        log::debug!("Allocation validation successful");
        Ok(true)
    }

    /// Validate a state transition (transfer) against Bitcoin
    pub async fn validate_transition(
        &self,
        from_utxo: &str,
        to_utxo: &str,
        amount: u64,
        bitcoin_txid: &str,
    ) -> Result<bool, ValidationError> {
        log::debug!("Validating transition: {} -> {} ({} sats)", 
                   from_utxo, to_utxo, amount);

        // 1. Parse UTXOs
        let (from_txid, from_vout) = self.parse_utxo(from_utxo)?;
        let (_to_txid, to_vout) = self.parse_utxo(to_utxo)?;

        // 2. Fetch Bitcoin transaction
        let bitcoin_tx = self.get_transaction(bitcoin_txid).await?;

        // 3. Verify input (from_utxo is spent)
        self.verify_input(&bitcoin_tx, &from_txid, from_vout)?;

        // 4. Verify output (to_utxo is created)
        self.verify_output(&bitcoin_tx, to_vout, amount)?;

        // 5. Check confirmations
        self.verify_confirmations(&bitcoin_tx).await?;

        log::debug!("Transition validation successful");
        Ok(true)
    }

    /// Get Bitcoin transaction by TXID using existing BalanceChecker infrastructure
    async fn get_transaction(&self, txid: &str) -> Result<BitcoinTransaction, ValidationError> {
        // Use the same HTTP client pattern as BalanceChecker
        let client = reqwest::Client::new();
        let url = format!("{}/tx/{}", self.esplora_url, txid);
        
        let response = client
            .get(&url)
            .send()
            .await
            .map_err(|e| ValidationError::Network(e.to_string()))?;

        if response.status().is_success() {
            let tx: BitcoinTransaction = response.json().await
                .map_err(|e| ValidationError::Parse(e.to_string()))?;
            Ok(tx)
        } else {
            Err(ValidationError::NotFound(txid.to_string()))
        }
    }

    /// Parse UTXO string into TXID and vout
    /// Expected format: "txid:vout" (e.g., "abc123...def:0")
    /// This matches the format used throughout the wallet codebase
    fn parse_utxo(&self, utxo: &str) -> Result<(String, u32), ValidationError> {
        // Validate input
        if utxo.is_empty() {
            return Err(ValidationError::InvalidUtxo);
        }

        // Split on colon - should have exactly 2 parts
        let parts: Vec<&str> = utxo.split(':').collect();
        
        match parts.len() {
            1 => {
                // Single part - assume vout=0 (common case for genesis UTXOs)
                let txid = parts[0].to_string();
                if txid.is_empty() {
                    return Err(ValidationError::InvalidUtxo);
                }
                log::debug!("Parsed UTXO '{}' as txid='{}', vout=0 (default)", utxo, txid);
                Ok((txid, 0))
            },
            2 => {
                // Two parts - txid:vout format
                let txid = parts[0].to_string();
                let vout_str = parts[1];
                
                if txid.is_empty() || vout_str.is_empty() {
                    return Err(ValidationError::InvalidUtxo);
                }
                
                let vout = vout_str.parse::<u32>()
                    .map_err(|_| {
                        log::warn!("Invalid vout '{}' in UTXO '{}'", vout_str, utxo);
                        ValidationError::InvalidUtxo
                    })?;
                
                log::debug!("Parsed UTXO '{}' as txid='{}', vout={}", utxo, txid, vout);
                Ok((txid, vout))
            },
            _ => {
                // More than 2 parts - invalid format
                log::warn!("Invalid UTXO format '{}' - too many colons", utxo);
                Err(ValidationError::InvalidUtxo)
            }
        }
    }

    /// Verify that a specific output exists in the transaction
    fn verify_output(
        &self,
        tx: &BitcoinTransaction,
        vout: u32,
        expected_amount: u64,
    ) -> Result<(), ValidationError> {
        if let Some(output) = tx.vout.iter().find(|out| out.n == vout) {
            if output.value == expected_amount {
                Ok(())
            } else {
                log::warn!("Amount mismatch: expected {}, got {}", expected_amount, output.value);
                Err(ValidationError::InvalidAmount)
            }
        } else {
            log::warn!("Output {} not found in transaction {}", vout, tx.txid);
            Err(ValidationError::OutputMismatch)
        }
    }

    /// Verify that a specific input exists in the transaction
    fn verify_input(
        &self,
        tx: &BitcoinTransaction,
        expected_txid: &str,
        expected_vout: u32,
    ) -> Result<(), ValidationError> {
        if let Some(_input) = tx.vin.iter().find(|inp| inp.txid == expected_txid && inp.vout == expected_vout) {
            Ok(())
        } else {
            log::warn!("Input {}:{} not found in transaction {}", expected_txid, expected_vout, tx.txid);
            Err(ValidationError::InputMismatch)
        }
    }

    /// Verify that the transaction has sufficient confirmations
    async fn verify_confirmations(&self, tx: &BitcoinTransaction) -> Result<(), ValidationError> {
        if !tx.status.confirmed {
            log::warn!("Transaction {} not confirmed", tx.txid);
            return Err(ValidationError::InsufficientConfirmations);
        }

        // Calculate actual confirmation count
        if let Some(block_height) = tx.status.block_height {
            let tip_height = self.get_tip_height().await?;
            let confirmations = tip_height.saturating_sub(block_height) + 1;
            
            if confirmations < self.min_confirmations {
                log::warn!("Transaction {} has only {} confirmations, need {}", 
                          tx.txid, confirmations, self.min_confirmations);
                return Err(ValidationError::InsufficientConfirmations);
            }
            
            log::debug!("Transaction {} has {} confirmations (required: {})", 
                       tx.txid, confirmations, self.min_confirmations);
            Ok(())
        } else {
            // Transaction is confirmed but no block height - this shouldn't happen
            log::warn!("Transaction {} confirmed but no block height", tx.txid);
            Err(ValidationError::InsufficientConfirmations)
        }
    }

    /// Get current blockchain tip height
    /// Reuses existing BalanceChecker infrastructure
    pub async fn get_tip_height(&self) -> Result<u32, ValidationError> {
        // Create a BalanceChecker instance to reuse existing tip height logic
        let balance_checker = BalanceChecker::new(self.esplora_url.clone());
        
        // Use existing get_tip_height method from BalanceChecker
        balance_checker.get_tip_height().await
            .map(|height| height as u32)
            .map_err(|e| ValidationError::Network(e.to_string()))
    }

    /// Check if a transaction is confirmed with sufficient confirmations
    pub async fn is_transaction_confirmed(
        &self,
        txid: &str,
        min_confirmations: Option<u32>,
    ) -> Result<bool, ValidationError> {
        let tx = self.get_transaction(txid).await?;
        
        if !tx.status.confirmed {
            return Ok(false);
        }

        let required_confirmations = min_confirmations.unwrap_or(self.min_confirmations);
        
        if let Some(block_height) = tx.status.block_height {
            let tip_height = self.get_tip_height().await?;
            let confirmations = tip_height - block_height + 1;
            Ok(confirmations >= required_confirmations)
        } else {
            Ok(false)
        }
    }
}
