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

