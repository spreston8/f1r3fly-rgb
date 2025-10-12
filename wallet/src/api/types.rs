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
    pub status: String,  // "genesis_imported", "pending", or "confirmed"
    pub import_type: String,  // "genesis" or "transfer"
    pub bitcoin_txid: Option<String>,  // None for genesis-only imports
}

#[derive(Debug, Serialize)]
pub struct ExportGenesisResponse {
    pub contract_id: String,
    pub consignment_filename: String,
    pub file_size_bytes: u64,
    pub download_url: String,
}
