// Firefly API request/response types

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
pub struct DeployRequest {
    pub term: String,           // Rholang code
    pub phlo_limit: i64,        // Execution limit
    pub language: String,       // "rholang"
}

#[derive(Debug, Deserialize)]
pub struct DeployResponse {
    pub deploy_id: String,
    pub status: String,
}

#[derive(Debug, Deserialize)]
pub struct ExploratoryDeployResponse {
    pub result: serde_json::Value,
    pub block_info: BlockInfo,
}

#[derive(Debug, Deserialize)]
pub struct BlockInfo {
    pub block_hash: String,
    pub block_number: i64,
}

#[derive(Debug, Deserialize)]
pub struct ProposeResponse {
    pub block_hash: String,
}

#[derive(Debug, Deserialize)]
pub struct DeployInfo {
    pub deploy_id: String,
    pub block_hash: Option<String>,
    pub status: String,
}

