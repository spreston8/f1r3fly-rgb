use blake2::{Blake2b, Digest};
use f1r3fly_models::casper::v1::deploy_response::Message as DeployResponseMessage;
use f1r3fly_models::casper::v1::deploy_service_client::DeployServiceClient;
use f1r3fly_models::casper::v1::propose_response::Message as ProposeResponseMessage;
use f1r3fly_models::casper::v1::propose_service_client::ProposeServiceClient;
use f1r3fly_models::casper::{BlocksQuery, DeployDataProto, LightBlockInfo, ProposeQuery};
use f1r3fly_models::ByteString;
use prost::Message;
use secp256k1::{Message as Secp256k1Message, Secp256k1, SecretKey};
use std::time::{SystemTime, UNIX_EPOCH};
use typenum::U32;

/// Bootstrap private key from standalone.yml
/// validator-private-key: 5f668a7ee96d944a4494cc947e4005e172d7ab3461ee5538f1f2a45a835e9657
const BOOTSTRAP_PRIVATE_KEY: &str =
    "5f668a7ee96d944a4494cc947e4005e172d7ab3461ee5538f1f2a45a835e9657";

pub struct FireflyClient {
    signing_key: SecretKey,
    node_host: String,
    grpc_port: u16,
}

impl FireflyClient {
    pub fn new(host: &str, grpc_port: u16) -> Self {
        let signing_key = SecretKey::from_slice(
            &hex::decode(BOOTSTRAP_PRIVATE_KEY).expect("Failed to decode bootstrap private key"),
        )
        .expect("Invalid bootstrap private key");

        Self {
            signing_key,
            node_host: host.to_string(),
            grpc_port,
        }
    }

    /// Deploy Rholang code (writes to blockchain)
    pub async fn deploy(&self, rholang_code: &str) -> Result<String, Box<dyn std::error::Error>> {
        // Get current block number for validity window
        let current_block = match self.get_current_block_number().await {
            Ok(block_num) => {
                log::debug!("Current Firefly block: {}", block_num);
                block_num
            }
            Err(e) => {
                log::warn!("Could not get current block number ({}), using VABN=0", e);
                0
            }
        };

        // Build and sign the deployment
        let deployment = self.build_deploy_msg(
            rholang_code.to_string(),
            5_000_000_000, // Large phlo limit for contracts
            "rholang".to_string(),
            current_block,
        );

        // Connect to the F1r3fly node via gRPC
        let mut deploy_service_client =
            DeployServiceClient::connect(format!("http://{}:{}/", self.node_host, self.grpc_port))
                .await?;

        // Send the deploy
        let deploy_response = deploy_service_client.do_deploy(deployment).await?;

        // Process the response
        let deploy_message = deploy_response
            .get_ref()
            .message
            .as_ref()
            .ok_or("Deploy result not found")?;

        match deploy_message {
            DeployResponseMessage::Error(service_error) => Err(service_error.clone().into()),
            DeployResponseMessage::Result(result) => {
                // Extract the deploy ID from the response
                let cleaned_result = result.trim();

                if let Some(deploy_id) = cleaned_result.strip_prefix("Success! DeployId is: ") {
                    Ok(deploy_id.trim().to_string())
                } else if let Some(deploy_id) =
                    cleaned_result.strip_prefix("Success!\nDeployId is: ")
                {
                    Ok(deploy_id.trim().to_string())
                } else if cleaned_result.starts_with("Success!") {
                    // Look for any long hex string in the response
                    let lines: Vec<&str> = cleaned_result.lines().collect();
                    for line in lines {
                        let trimmed = line.trim();
                        if trimmed.len() > 64 && trimmed.chars().all(|c| c.is_ascii_hexdigit()) {
                            return Ok(trimmed.to_string());
                        }
                    }
                    Err(format!("Could not extract deploy ID from response: {}", result).into())
                } else {
                    Ok(cleaned_result.to_string())
                }
            }
        }
    }

    /// Propose block
    pub async fn propose(&self) -> Result<String, Box<dyn std::error::Error>> {
        // Connect to the F1r3fly node's propose service
        let mut propose_client =
            ProposeServiceClient::connect(format!("http://{}:{}/", self.node_host, self.grpc_port))
                .await?;

        // Send the propose request
        let propose_response = propose_client
            .propose(ProposeQuery { is_async: false })
            .await?
            .into_inner();

        // Process the response
        let message = propose_response.message.ok_or("Missing propose response")?;

        match message {
            ProposeResponseMessage::Result(block_hash) => {
                // Extract the block hash from the response
                if let Some(hash) = block_hash
                    .strip_prefix("Success! Block ")
                    .and_then(|s| s.strip_suffix(" created and added."))
                {
                    Ok(hash.to_string())
                } else {
                    Ok(block_hash)
                }
            }
            ProposeResponseMessage::Error(error) => {
                Err(format!("Propose error: {:?}", error).into())
            }
        }
    }

    /// Wait for deploy to be included in a block
    pub async fn wait_for_deploy(
        &self,
        deploy_id: &str,
        max_attempts: u32,
    ) -> Result<String, Box<dyn std::error::Error>> {
        for attempt in 1..=max_attempts {
            let response = reqwest::Client::new()
                .get(&format!(
                    "http://{}:40403/api/deploy/{}",
                    self.node_host, deploy_id
                ))
                .send()
                .await;

            match response {
                Ok(resp) if resp.status().is_success() => {
                    let deploy_info: serde_json::Value = resp.json().await?;
                    if let Some(block_hash) = deploy_info.get("blockHash").and_then(|v| v.as_str())
                    {
                        return Ok(block_hash.to_string());
                    }
                }
                _ => {
                    // Deploy not yet included, continue waiting
                }
            }

            if attempt < max_attempts {
                tokio::time::sleep(std::time::Duration::from_secs(2)).await;
            }
        }

        Err("Deploy not included in block after max attempts".into())
    }

    /// Gets the current block number from the blockchain
    async fn get_current_block_number(&self) -> Result<i64, Box<dyn std::error::Error>> {
        // Get the most recent block using show_main_chain with depth 1
        let blocks = self.show_main_chain(1).await?;

        if let Some(latest_block) = blocks.first() {
            Ok(latest_block.block_number)
        } else {
            // Fallback to 0 if no blocks found (genesis case)
            Ok(0)
        }
    }

    /// Gets blocks in the main chain
    async fn show_main_chain(
        &self,
        depth: u32,
    ) -> Result<Vec<LightBlockInfo>, Box<dyn std::error::Error>> {
        use f1r3fly_models::casper::v1::block_info_response::Message;

        // Connect to the F1r3fly node
        let mut deploy_service_client =
            DeployServiceClient::connect(format!("http://{}:{}/", self.node_host, self.grpc_port))
                .await?;

        // Create the query
        let query = BlocksQuery {
            depth: depth as i32,
        };

        // Send the query and collect streaming response
        let mut stream = deploy_service_client
            .show_main_chain(query)
            .await?
            .into_inner();

        let mut blocks = Vec::new();
        while let Some(response) = stream.message().await? {
            if let Some(message) = response.message {
                match message {
                    Message::Error(service_error) => {
                        return Err(
                            format!("gRPC Error: {}", service_error.messages.join("; ")).into()
                        );
                    }
                    Message::BlockInfo(block_info) => {
                        blocks.push(block_info);
                    }
                }
            }
        }

        Ok(blocks)
    }

    /// Builds and signs a deploy message
    ///
    /// This follows the exact same signing process as rust-client:
    /// 1. Create DeployDataProto with deployment parameters
    /// 2. Serialize it (excluding language, sig, deployer, sig_algorithm fields)
    /// 3. Hash with Blake2b-256
    /// 4. Sign with secp256k1 using the bootstrap private key
    /// 5. Add the signature and public key to the complete DeployDataProto
    fn build_deploy_msg(
        &self,
        code: String,
        phlo_limit: i64,
        language: String,
        valid_after_block_number: i64,
    ) -> DeployDataProto {
        // Get current timestamp in milliseconds
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Failed to get system time")
            .as_millis() as i64;

        // Create a projection with only the fields used for signature calculation
        // IMPORTANT: The language field is deliberately excluded from signature calculation
        let projection = DeployDataProto {
            term: code.clone(),
            timestamp,
            phlo_price: 1,
            phlo_limit,
            valid_after_block_number,
            shard_id: "root".into(),
            language: String::new(), // Excluded from signature calculation
            sig: ByteString::new(),
            deployer: ByteString::new(),
            sig_algorithm: String::new(),
        };

        // Serialize the projection for hashing
        let serialized = projection.encode_to_vec();

        // Hash with Blake2b256
        let digest = blake2b_256_hash(&serialized);

        // Sign the digest with secp256k1
        let secp = Secp256k1::new();
        let message = Secp256k1Message::from_digest(digest.into());
        let signature = secp.sign_ecdsa(&message, &self.signing_key);

        // Get signature in DER format
        let sig_bytes = signature.serialize_der().to_vec();

        // Get the public key in uncompressed format
        let public_key = self.signing_key.public_key(&secp);
        let pub_key_bytes = public_key.serialize_uncompressed().to_vec();

        // Return the complete deploy message
        DeployDataProto {
            term: code,
            timestamp,
            phlo_price: 1,
            phlo_limit,
            valid_after_block_number,
            shard_id: "root".into(),
            language,
            sig: ByteString::from(sig_bytes),
            sig_algorithm: "secp256k1".into(),
            deployer: ByteString::from(pub_key_bytes),
        }
    }
}

/// Computes a Blake2b 256-bit hash of the provided data
fn blake2b_256_hash(data: &[u8]) -> [u8; 32] {
    let mut blake = Blake2b::<U32>::new();
    blake.update(data);
    let hash = blake.finalize();
    let mut result = [0u8; 32];
    result.copy_from_slice(&hash);
    result
}
