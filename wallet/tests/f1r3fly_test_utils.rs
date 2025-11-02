use std::sync::OnceLock;
use std::time::Duration;

use chrono::Utc;
use secp256k1::{PublicKey, Secp256k1, SecretKey};

use wallet::firefly::client::{FireflyClient, RgbStorageUris};
use wallet::firefly::registry;
use wallet::firefly::types::*;

/// F1r3fly test configuration
#[derive(Debug, Clone)]
pub struct FireflyTestConfig {
    pub host: String,
    pub grpc_port: u16,
    pub http_port: u16,
}

impl FireflyTestConfig {
    pub fn from_env() -> Self {
        let host = std::env::var("FIREFLY_TEST_HOST").unwrap_or_else(|_| "localhost".to_string());

        let grpc_port = std::env::var("FIREFLY_TEST_GRPC_PORT")
            .unwrap_or_else(|_| "40401".to_string())
            .parse()
            .expect("Invalid FIREFLY_TEST_GRPC_PORT");

        let http_port = std::env::var("FIREFLY_TEST_HTTP_PORT")
            .unwrap_or_else(|_| "40403".to_string())
            .parse()
            .expect("Invalid FIREFLY_TEST_HTTP_PORT");

        log::info!(
            "🔥 F1r3fly Test Node: {}:{} (gRPC), :{} (HTTP)",
            host,
            grpc_port,
            http_port
        );

        Self {
            host,
            grpc_port,
            http_port,
        }
    }

    pub fn create_client(&self) -> FireflyClient {
        FireflyClient::new(&self.host, self.grpc_port, self.http_port)
    }
}

pub fn sample_contract_metadata() -> ContractMetadata {
    ContractMetadata {
        ticker: "TEST".to_string(),
        name: "Test Token".to_string(),
        precision: 8,
        total_supply: 1_000_000,
        genesis_txid: "0000000000000000000000000000000000000000000000000000000000000000"
            .to_string(),
        issuer_pubkey: "test_issuer_pubkey".to_string(),
    }
}

pub fn sample_contract_with_ticker(
    ticker: &str,
    name: &str,
    total_supply: u64,
) -> ContractMetadata {
    ContractMetadata {
        ticker: ticker.to_string(),
        name: name.to_string(),
        precision: 8,
        total_supply,
        genesis_txid: "0000000000000000000000000000000000000000000000000000000000000000"
            .to_string(),
        issuer_pubkey: "test_issuer_pubkey".to_string(),
    }
}

pub fn generate_test_contract_id(test_name: &str) -> String {
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    format!("rgb20_test_{}_{}", test_name, timestamp)
}

pub fn sample_allocation_data() -> (String, String, u64) {
    let utxo = "0000000000000000000000000000000000000000000000000000000000000000:0".to_string();
    let owner_pubkey =
        "02a1b2c3d4e5f6a7b8c9d0e1f2a3b4c5d6e7f8a9b0c1d2e3f4a5b6c7d8e9f0a1b2".to_string();
    let amount = 50_000u64;
    (utxo, owner_pubkey, amount)
}

pub fn sample_transition_data() -> (String, String, u64) {
    let from_utxo =
        "source_utxo_0000000000000000000000000000000000000000000000000000000000:0".to_string();
    let to_utxo =
        "dest_utxo_1111111111111111111111111111111111111111111111111111111111:0".to_string();
    let amount = 30_000u64;
    (from_utxo, to_utxo, amount)
}

pub async fn store_test_contract(
    client: &FireflyClient,
    contract_id: &str,
    metadata: &ContractMetadata,
) -> anyhow::Result<(String, String)> {
    let deploy_id = client
        .store_contract(contract_id, metadata.clone())
        .await
        .map_err(|e| anyhow::anyhow!("Store contract failed: {}", e))?;

    let block_hash = client
        .wait_for_deploy(&deploy_id, 60)
        .await
        .map_err(|e| anyhow::anyhow!("Wait for deploy failed: {}", e))?;

    client
        .wait_for_block_finalization(&block_hash, 24)
        .await
        .map_err(|e| anyhow::anyhow!("Wait for finalization failed: {}", e))?;

    Ok((deploy_id, block_hash))
}

pub async fn store_test_allocation(
    client: &FireflyClient,
    contract_id: &str,
    utxo: &str,
    owner_pubkey: &str,
    amount: u64,
    bitcoin_txid: &str,
) -> anyhow::Result<(String, String)> {
    let deploy_id = client
        .store_allocation(contract_id, utxo, owner_pubkey, amount, bitcoin_txid)
        .await
        .map_err(|e| anyhow::anyhow!("Store allocation failed: {}", e))?;

    let block_hash = client
        .wait_for_deploy(&deploy_id, 60)
        .await
        .map_err(|e| anyhow::anyhow!("Wait for deploy failed: {}", e))?;

    client
        .wait_for_block_finalization(&block_hash, 24)
        .await
        .map_err(|e| anyhow::anyhow!("Wait for finalization failed: {}", e))?;

    Ok((deploy_id, block_hash))
}

pub async fn store_test_transition(
    client: &FireflyClient,
    contract_id: &str,
    from_utxo: &str,
    to_utxo: &str,
    amount: u64,
    bitcoin_txid: &str,
) -> anyhow::Result<(String, String)> {
    let deploy_id = client
        .record_transition(contract_id, from_utxo, to_utxo, amount, bitcoin_txid)
        .await
        .map_err(|e| anyhow::anyhow!("Store transition failed: {}", e))?;

    let block_hash = client
        .wait_for_deploy(&deploy_id, 60)
        .await
        .map_err(|e| anyhow::anyhow!("Wait for deploy failed: {}", e))?;

    client
        .wait_for_block_finalization(&block_hash, 24)
        .await
        .map_err(|e| anyhow::anyhow!("Wait for finalization failed: {}", e))?;

    Ok((deploy_id, block_hash))
}

pub async fn check_firefly_node_health(config: &FireflyTestConfig) -> anyhow::Result<()> {
    let url = format!("http://{}:{}/api/status", config.host, config.http_port);

    let client = reqwest::Client::builder()
        .no_proxy()
        .timeout(Duration::from_secs(5))
        .build()
        .map_err(|e| anyhow::anyhow!("Failed to create HTTP client: {}", e))?;

    let response = client
        .get(&url)
        .send()
        .await
        .map_err(|e| anyhow::anyhow!("F1r3fly node unreachable: {}", e))?;

    if response.status().is_success() {
        Ok(())
    } else {
        Err(anyhow::anyhow!(
            "F1r3fly node returned status: {}",
            response.status()
        ))
    }
}

static RGB_STORAGE_URIS: OnceLock<RgbStorageUris> = OnceLock::new();

pub async fn ensure_rgb_storage_deployed(
    client: &FireflyClient,
) -> anyhow::Result<&'static RgbStorageUris> {
    if let Some(uris) = RGB_STORAGE_URIS.get() {
        return Ok(uris);
    }

    let private_key =
        std::env::var("FIREFLY_PRIVATE_KEY").expect("FIREFLY_PRIVATE_KEY must be set in .env file");

    let (rholang_code, timestamp_millis) = generate_rgb_storage_rholang(&private_key)?;

    let deploy_id = client
        .deploy_with_timestamp(&rholang_code, timestamp_millis)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to deploy RGB storage contract: {}", e))?;

    let _block_hash = client
        .wait_for_deploy(&deploy_id, 60)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to wait for deploy: {}", e))?;

    let uri = registry::compute_registry_uri_from_private_key(&private_key)
        .map_err(|e| anyhow::anyhow!("Failed to compute URI: {}", e))?;

    let uris = RgbStorageUris {
        store_contract: uri.clone(),
        get_contract: uri.clone(),
        search_by_ticker: uri.clone(),
        store_allocation: uri.clone(),
        get_allocation: uri.clone(),
        record_transition: uri.clone(),
        get_transition: uri,
    };

    let cached_uris = RGB_STORAGE_URIS.get_or_init(|| uris);

    Ok(cached_uris)
}

fn generate_rgb_storage_rholang(private_key_hex: &str) -> anyhow::Result<(String, i64)> {
    let secp = Secp256k1::new();

    let secret_key_bytes = hex::decode(private_key_hex)?;
    let secret_key = SecretKey::from_slice(&secret_key_bytes)?;
    let public_key = PublicKey::from_secret_key(&secp, &secret_key);

    let timestamp = Utc::now();
    let timestamp_millis = timestamp.timestamp_millis();
    let version = timestamp_millis;

    let signature = registry::generate_insert_signed_signature(&secret_key, timestamp, &public_key, version);
    let uri = registry::public_key_to_uri(&public_key);

    let public_key_hex = hex::encode(public_key.serialize_uncompressed());
    let signature_hex = hex::encode(&signature);

    let template = include_str!("../rholang/rgb_state_storage.rho");

    let rholang_code = template
        .replace("{{URI}}", &uri)
        .replace("{{PUBLIC_KEY}}", &public_key_hex)
        .replace("{{VERSION}}", &version.to_string())
        .replace("{{SIGNATURE}}", &signature_hex);

    Ok((rholang_code, timestamp_millis))
}
