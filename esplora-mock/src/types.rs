/// Esplora API response types
/// 
/// These types match the Esplora API format so clients can consume them transparently.

use serde::{Deserialize, Serialize};

/// UTXO response from /address/{address}/utxo
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UtxoResponse {
    pub txid: String,
    pub vout: u32,
    pub value: u64,
    pub status: UtxoStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UtxoStatus {
    pub confirmed: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub block_height: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub block_hash: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub block_time: Option<u64>,
}

/// Transaction status response from /tx/{txid}/status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TxStatusResponse {
    pub confirmed: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub block_height: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub block_hash: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub block_time: Option<u64>,
}

/// Transaction response from /tx/{txid}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TxResponse {
    pub txid: String,
    pub version: i32,
    pub locktime: u32,
    pub vin: Vec<TxInput>,
    pub vout: Vec<TxOutput>,
    pub size: usize,
    pub weight: usize,
    pub fee: u64,
    pub status: TxStatusResponse,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TxInput {
    pub txid: String,
    pub vout: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prevout: Option<TxOutput>,
    pub scriptsig: String,
    pub scriptsig_asm: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub witness: Option<Vec<String>>,
    pub is_coinbase: bool,
    pub sequence: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TxOutput {
    pub scriptpubkey: String,
    pub scriptpubkey_asm: String,
    pub scriptpubkey_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scriptpubkey_address: Option<String>,
    pub value: u64,
}

/// Output spending status from /tx/{txid}/outspend/{index}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputSpendStatus {
    pub spent: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub txid: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vin: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<TxStatusResponse>,
}

