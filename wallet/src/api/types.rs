use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct CreateWalletRequest {
    pub name: String,
}

#[derive(Debug, Deserialize)]
pub struct ImportWalletRequest {
    pub name: String,
    pub mnemonic: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CreateUtxoRequest {
    pub amount_btc: Option<f64>,
    pub fee_rate_sat_vb: Option<u64>,
}

#[derive(Debug, Serialize)]
pub struct CreateUtxoResponse {
    pub txid: String,
    pub amount_sats: u64,
    pub fee_sats: u64,
    pub target_address: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct UnlockUtxoRequest {
    pub txid: String,
    pub vout: u32,
    pub fee_rate_sat_vb: Option<u64>,
}

#[derive(Debug, Serialize)]
pub struct UnlockUtxoResponse {
    pub txid: String,
    pub recovered_sats: u64,
    pub fee_sats: u64,
}

#[derive(Debug, Deserialize)]
pub struct SendBitcoinRequest {
    pub to_address: String,
    pub amount_sats: u64,
    pub fee_rate_sat_vb: Option<u64>,
}

#[derive(Debug, Serialize)]
pub struct SendBitcoinResponse {
    pub txid: String,
    pub amount_sats: u64,
    pub fee_sats: u64,
    pub to_address: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct GenerateInvoiceRequest {
    pub contract_id: String,
    pub amount: Option<u64>,
}

#[derive(Debug, Serialize)]
pub struct GenerateInvoiceResponse {
    pub invoice: String,
    pub contract_id: String,
    pub amount: Option<u64>,
    pub seal_utxo: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct SendTransferRequest {
    pub invoice: String,
    pub fee_rate_sat_vb: Option<u64>,
}

#[derive(Debug, Serialize)]
pub struct SendTransferResponse {
    pub bitcoin_txid: String,
    pub consignment_download_url: String,
    pub consignment_filename: String,
    pub status: String,
}

#[derive(Debug, Serialize)]
pub struct AcceptConsignmentResponse {
    pub contract_id: String,
    pub status: String,      // "genesis_imported", "pending", or "confirmed"
    pub import_type: String, // "genesis" or "transfer"
    pub bitcoin_txid: Option<String>, // None for genesis-only imports
}

#[derive(Debug, Serialize)]
pub struct ExportGenesisResponse {
    pub contract_id: String,
    pub consignment_filename: String,
    pub file_size_bytes: u64,
    pub download_url: String,
}

#[derive(Debug, Serialize)]
pub struct DeleteWalletResponse {
    pub wallet_name: String,
    pub status: String,
}

// Firefly integration types
#[derive(Debug, Serialize, Deserialize)]
pub struct FireflyNodeStatus {
    pub node_connected: bool,
    pub node_url: String,
    pub peers: Option<u64>,
    pub version: Option<String>,
    pub message: String,
}

// Wallet management result types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletInfo {
    pub name: String,
    pub mnemonic: String,
    pub first_address: String,
    pub public_address: String,
    pub descriptor: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletMetadata {
    pub name: String,
    pub created_at: String,
    pub last_synced: Option<String>,
}

// Address management result types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddressInfo {
    pub index: u32,
    pub address: String,
    pub used: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NextAddressInfo {
    pub address: String,
    pub index: u32,
    pub total_used: usize,
    pub descriptor: String,
}

// Sync result types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncResult {
    pub synced_height: u64,
    pub addresses_checked: u32,
    pub new_transactions: usize,
}

// Bitcoin operation result types  
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateUtxoResult {
    pub txid: String,
    pub amount_sats: u64,
    pub fee_sats: u64,
    pub target_address: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnlockUtxoResult {
    pub txid: String,
    pub recovered_sats: u64,
    pub fee_sats: u64,
}

// RGB operation result types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerateInvoiceResult {
    pub invoice: String,
    pub contract_id: String,
    pub amount: Option<u64>,
    pub seal_utxo: String,
}
