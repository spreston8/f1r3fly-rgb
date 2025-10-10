use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct CreateWalletRequest {
    pub name: String,
}

#[derive(Debug, Deserialize)]
pub struct ImportWalletRequest {
    pub name: String,
    pub mnemonic: String,
}

