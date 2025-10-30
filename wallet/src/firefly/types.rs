// F1r3fly State Storage Types
// Data structures for RSpace++ state storage operations

use serde::{Deserialize, Serialize};

// ============================================================================
// Contract Metadata Types
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractMetadata {
    pub ticker: String,
    pub name: String,
    pub precision: u8,
    pub total_supply: u64,
    pub genesis_txid: String,
    pub issuer_pubkey: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Contract {
    pub schema: String,
    pub ticker: String,
    pub name: String,
    pub precision: u8,
    pub total_supply: u64,
    pub genesis_txid: String,
    pub issuer: String,
    pub created_at: i64,
    pub network: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractData {
    pub success: bool,
    pub contract: Option<Contract>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractSearchResult {
    pub success: bool,
    pub contract_id: Option<String>,
    pub contract: Option<Contract>,
    pub error: Option<String>,
}

// ============================================================================
// Allocation Types
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Allocation {
    pub owner: String,
    pub amount: u64,
    pub bitcoin_txid: String,
    pub confirmed: bool,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AllocationData {
    pub success: bool,
    pub allocation: Option<Allocation>,
    pub error: Option<String>,
}

// ============================================================================
// Transition Types
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transition {
    pub from: String,
    pub to: String,
    pub amount: u64,
    pub bitcoin_txid: String,
    pub timestamp: i64,
    pub validated: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransitionData {
    pub success: bool,
    pub transition: Option<Transition>,
    pub error: Option<String>,
}

// ============================================================================
// Generic Response Types
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenericResponse {
    pub success: bool,
    pub data: Option<serde_json::Value>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoreResponse {
    pub success: bool,
    pub id: Option<String>,
    pub error: Option<String>,
}