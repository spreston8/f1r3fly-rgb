use super::addresses::AddressManager;
use super::balance::BalanceChecker;
use super::keys::KeyManager;
use super::rgb::RgbManager;
use super::rgb_runtime::RgbRuntimeManager;
use super::signer::WalletSigner;
use super::storage::{Metadata, Storage};
use bitcoin::Network;
use chrono::Utc;
use std::str::FromStr;

pub struct WalletManager {
    pub storage: Storage,
    balance_checker: BalanceChecker,
    pub rgb_manager: RgbManager,
    rgb_runtime_manager: RgbRuntimeManager,
}

impl WalletManager {
    pub fn new() -> Self {
        let storage = Storage::new();
        let rgb_data_dir = storage.base_dir().join("rgb_data");
        let rgb_manager = RgbManager::new(rgb_data_dir, bpstd::Network::Signet)
            .expect("Failed to initialize RGB manager");
        
        let rgb_runtime_manager = RgbRuntimeManager::new(
            storage.base_dir().clone(),
            bpstd::Network::Signet,
        );
        
        Self {
            storage,
            balance_checker: BalanceChecker::new(),
            rgb_manager,
            rgb_runtime_manager,
        }
    }

    pub fn create_wallet(&self, name: &str) -> Result<WalletInfo, crate::error::WalletError> {
        if self.storage.wallet_exists(name) {
            return Err(crate::error::WalletError::WalletExists(name.to_string()));
        }

        let keys = KeyManager::generate()?;

        self.storage.create_wallet(name)?;

        let metadata = Metadata {
            name: name.to_string(),
            created_at: Utc::now(),
            network: "signet".to_string(),
        };
        self.storage.save_metadata(name, &metadata)?;
        self.storage.save_mnemonic(name, &keys.mnemonic)?;
        self.storage.save_descriptor(name, &keys.descriptor)?;

        let first_address = AddressManager::derive_address(&keys.descriptor, 0, Network::Signet)?;

        Ok(WalletInfo {
            name: name.to_string(),
            mnemonic: keys.mnemonic.to_string(),
            first_address: first_address.to_string(),
            descriptor: keys.descriptor,
        })
    }

    pub fn import_wallet(
        &self,
        name: &str,
        mnemonic: &str,
    ) -> Result<WalletInfo, crate::error::WalletError> {
        if self.storage.wallet_exists(name) {
            return Err(crate::error::WalletError::WalletExists(name.to_string()));
        }

        let keys = KeyManager::from_mnemonic(mnemonic)?;

        self.storage.create_wallet(name)?;

        let metadata = Metadata {
            name: name.to_string(),
            created_at: Utc::now(),
            network: "signet".to_string(),
        };
        self.storage.save_metadata(name, &metadata)?;
        self.storage.save_mnemonic(name, &keys.mnemonic)?;
        self.storage.save_descriptor(name, &keys.descriptor)?;

        let first_address = AddressManager::derive_address(&keys.descriptor, 0, Network::Signet)?;

        Ok(WalletInfo {
            name: name.to_string(),
            mnemonic: keys.mnemonic.to_string(),
            first_address: first_address.to_string(),
            descriptor: keys.descriptor,
        })
    }

    pub fn list_wallets(&self) -> Result<Vec<WalletMetadata>, crate::error::WalletError> {
        let wallet_names = self.storage.list_wallets()?;
        let mut wallets = Vec::new();

        for name in wallet_names {
            if let Ok(metadata) = self.storage.load_metadata(&name) {
                let state = self.storage.load_state(&name).ok();
                let last_synced = state
                    .and_then(|s| s.last_synced_height)
                    .map(|h| format!("Height: {}", h));

                wallets.push(WalletMetadata {
                    name: metadata.name,
                    created_at: metadata.created_at.to_rfc3339(),
                    last_synced,
                });
            }
        }

        Ok(wallets)
    }

    pub fn get_addresses(
        &self,
        name: &str,
        count: u32,
    ) -> Result<Vec<AddressInfo>, crate::error::WalletError> {
        if !self.storage.wallet_exists(name) {
            return Err(crate::error::WalletError::WalletNotFound(name.to_string()));
        }

        let descriptor = self.storage.load_descriptor(name)?;
        let state = self.storage.load_state(name)?;

        let addresses =
            AddressManager::derive_addresses(&descriptor, 0, count, Network::Signet)?;

        let address_infos = addresses
            .into_iter()
            .map(|(index, address)| AddressInfo {
                index,
                address: address.to_string(),
                used: state.used_addresses.contains(&index),
            })
            .collect();

        Ok(address_infos)
    }

    pub fn get_primary_address(
        &self,
        name: &str,
    ) -> Result<NextAddressInfo, crate::error::WalletError> {
        if !self.storage.wallet_exists(name) {
            return Err(crate::error::WalletError::WalletNotFound(name.to_string()));
        }

        let descriptor = self.storage.load_descriptor(name)?;
        let state = self.storage.load_state(name)?;

        // Always return Address #0 for consistent development experience
        let primary_index = 0;

        let address = AddressManager::derive_address(&descriptor, primary_index, Network::Signet)?;

        Ok(NextAddressInfo {
            address: address.to_string(),
            index: primary_index,
            total_used: state.used_addresses.len(),
            descriptor,
        })
    }

    pub async fn get_balance(
        &self,
        name: &str,
    ) -> Result<super::balance::BalanceInfo, crate::error::WalletError> {
        if !self.storage.wallet_exists(name) {
            return Err(crate::error::WalletError::WalletNotFound(name.to_string()));
        }

        let descriptor = self.storage.load_descriptor(name)?;

        const GAP_LIMIT: u32 = 20;
        let addresses_with_indices =
            AddressManager::derive_addresses(&descriptor, 0, GAP_LIMIT, Network::Signet)?;

        let mut balance = self.balance_checker.calculate_balance(&addresses_with_indices).await?;

        // Check each UTXO for RGB assets
        for utxo in &mut balance.utxos {
            let txid = match utxo.txid.parse::<bitcoin::Txid>() {
                Ok(t) => t,
                Err(_) => continue,
            };

            // Check if UTXO is occupied by RGB assets
            if let Ok(is_occupied) = self.rgb_manager.check_utxo_occupied(txid, utxo.vout) {
                utxo.is_occupied = is_occupied;

                // If occupied, get the bound assets
                if is_occupied {
                    if let Ok(assets) = self.rgb_manager.get_bound_assets(txid, utxo.vout) {
                        utxo.bound_assets = assets;
                    }
                }
            }
        }

        Ok(balance)
    }

    pub async fn sync_wallet(
        &self,
        name: &str,
    ) -> Result<SyncResult, crate::error::WalletError> {
        if !self.storage.wallet_exists(name) {
            return Err(crate::error::WalletError::WalletNotFound(name.to_string()));
        }

        let descriptor = self.storage.load_descriptor(name)?;
        let mut state = self.storage.load_state(name)?;

        let tip_height = self.balance_checker.get_tip_height().await?;

        const GAP_LIMIT: u32 = 20;
        let addresses =
            AddressManager::derive_addresses(&descriptor, 0, GAP_LIMIT, Network::Signet)?;

        let mut new_transactions = 0;

        for (index, address) in addresses {
            let utxos = self.balance_checker.get_address_utxos(&address, index).await?;
            if !utxos.is_empty() && !state.used_addresses.contains(&index) {
                state.used_addresses.push(index);
                new_transactions += utxos.len();
            }
        }

        state.last_synced_height = Some(tip_height);
        self.storage.save_state(name, &state)?;

        Ok(SyncResult {
            synced_height: tip_height,
            addresses_checked: GAP_LIMIT,
            new_transactions,
        })
    }

    pub async fn create_utxo(
        &self,
        name: &str,
        request: CreateUtxoRequest,
    ) -> Result<CreateUtxoResult, crate::error::WalletError> {
        if !self.storage.wallet_exists(name) {
            return Err(crate::error::WalletError::WalletNotFound(name.to_string()));
        }

        let amount_sats = match request.amount_btc {
            Some(btc) => (btc * 100_000_000.0) as u64,
            None => 30_000,
        };

        let fee_rate = request.fee_rate_sat_vb.unwrap_or(2);

        let balance = self.get_balance(name).await?;
        
        if balance.utxos.is_empty() {
            return Err(crate::error::WalletError::InsufficientFunds(
                "No UTXOs available to create new UTXO".to_string()
            ));
        }

        let descriptor = self.storage.load_descriptor(name)?;
        let mut state = self.storage.load_state(name)?;

        let mut next_index = 0;
        while state.used_addresses.contains(&next_index) {
            next_index += 1;
        }
        state.used_addresses.push(next_index);

        let recipient_address = AddressManager::derive_address(&descriptor, next_index, Network::Signet)?;

        let tx_builder = super::transaction::TransactionBuilder::new(Network::Signet);
        
        let tx = tx_builder.build_send_to_self(
            &balance.utxos,
            amount_sats,
            fee_rate,
            recipient_address.clone(),
        )?;

        let mnemonic = self.storage.load_mnemonic(name)?;

        // Sign transaction with correct keys for each UTXO's address index
        let signed_tx = self.sign_transaction_multi_key(tx, &balance.utxos, &mnemonic)?;

        let txid = super::transaction::broadcast_transaction(&signed_tx, Network::Signet).await?;

        let total_input: u64 = balance.utxos.iter()
            .filter(|u| {
                signed_tx.input.iter().any(|input| {
                    if let Ok(tid) = u.txid.parse::<bitcoin::Txid>() {
                        tid == input.previous_output.txid && u.vout == input.previous_output.vout
                    } else {
                        false
                    }
                })
            })
            .map(|u| u.amount_sats)
            .sum();
        
        let fee_sats = total_input - signed_tx.output.iter().map(|o| o.value.to_sat()).sum::<u64>();

        self.storage.save_state(name, &state)?;

        Ok(CreateUtxoResult {
            txid,
            amount_sats,
            fee_sats,
            target_address: recipient_address.to_string(),
        })
    }

    pub async fn unlock_utxo(
        &self,
        name: &str,
        request: UnlockUtxoRequest,
    ) -> Result<UnlockUtxoResult, crate::error::WalletError> {
        if !self.storage.wallet_exists(name) {
            return Err(crate::error::WalletError::WalletNotFound(name.to_string()));
        }

        let fee_rate = request.fee_rate_sat_vb.unwrap_or(2);

        let balance = self.get_balance(name).await?;

        let target_utxo = balance.utxos.iter()
            .find(|u| u.txid == request.txid && u.vout == request.vout)
            .ok_or_else(|| crate::error::WalletError::Internal(
                format!("UTXO {}:{} not found", request.txid, request.vout)
            ))?
            .clone();

        let descriptor = self.storage.load_descriptor(name)?;
        let mut state = self.storage.load_state(name)?;

        let mut next_index = 0;
        while state.used_addresses.contains(&next_index) {
            next_index += 1;
        }
        state.used_addresses.push(next_index);

        let destination_address = AddressManager::derive_address(&descriptor, next_index, Network::Signet)?;

        let tx_builder = super::transaction::TransactionBuilder::new(Network::Signet);
        
        let tx = tx_builder.build_unlock_utxo_tx(
            &target_utxo,
            destination_address.clone(),
            fee_rate,
        )?;

        let mnemonic = self.storage.load_mnemonic(name)?;

        // Sign transaction with correct key for the UTXO's address index
        let signed_tx = self.sign_transaction_multi_key(tx, &vec![target_utxo.clone()], &mnemonic)?;

        let txid = super::transaction::broadcast_transaction(&signed_tx, Network::Signet).await?;

        let fee_sats = target_utxo.amount_sats - signed_tx.output[0].value.to_sat();
        let recovered_sats = signed_tx.output[0].value.to_sat();

        self.storage.save_state(name, &state)?;

        Ok(UnlockUtxoResult {
            txid,
            recovered_sats,
            fee_sats,
        })
    }
    
    /// Helper method to initialize RGB Runtime for a wallet (used by transfer APIs)
    pub(crate) fn get_runtime(
        &self,
        wallet_name: &str,
    ) -> Result<rgbp::RgbpRuntimeDir<rgbp::resolvers::MultiResolver>, crate::error::WalletError> {
        self.rgb_runtime_manager.init_runtime(wallet_name)
    }
    
    pub(crate) fn get_runtime_no_sync(
        &self,
        wallet_name: &str,
    ) -> Result<rgbp::RgbpRuntimeDir<rgbp::resolvers::MultiResolver>, crate::error::WalletError> {
        self.rgb_runtime_manager.init_runtime_no_sync(wallet_name)
    }
    
    /// Helper method to derive private key for a specific address index
    fn derive_private_key_for_index(
        &self,
        mnemonic: &bip39::Mnemonic,
        address_index: u32,
    ) -> Result<bitcoin::PrivateKey, crate::error::WalletError> {
        let seed = mnemonic.to_seed("");
        let master_key = bitcoin::bip32::Xpriv::new_master(Network::Signet, &seed)
            .map_err(|e| crate::error::WalletError::Bitcoin(e.to_string()))?;

        let path = bitcoin::bip32::DerivationPath::from_str(&format!("m/84'/1'/0'/0/{}", address_index))
            .map_err(|e| crate::error::WalletError::Bitcoin(e.to_string()))?;
        
        let derived_key = master_key.derive_priv(&bitcoin::secp256k1::Secp256k1::new(), &path)
            .map_err(|e| crate::error::WalletError::Bitcoin(e.to_string()))?;

        Ok(bitcoin::PrivateKey::new(derived_key.private_key, Network::Signet))
    }
    
    /// Sign a transaction using the correct private keys for each UTXO
    fn sign_transaction_multi_key(
        &self,
        tx: bitcoin::Transaction,
        utxos: &[super::balance::UTXO],
        mnemonic: &bip39::Mnemonic,
    ) -> Result<bitcoin::Transaction, crate::error::WalletError> {
        use bitcoin::sighash::{EcdsaSighashType, SighashCache};
        use bitcoin::secp256k1::{Message, Secp256k1};
        use bitcoin::hashes::Hash;
        use bitcoin::PublicKey;
        
        let mut signed_tx = tx.clone();
        let secp = Secp256k1::new();

        for (input_index, input) in tx.input.iter().enumerate() {
            // Find the UTXO for this input
            let utxo = utxos.iter()
                .find(|u| {
                    if let Ok(txid) = u.txid.parse::<bitcoin::Txid>() {
                        txid == input.previous_output.txid && u.vout == input.previous_output.vout
                    } else {
                        false
                    }
                })
                .ok_or_else(|| crate::error::WalletError::Bitcoin("UTXO not found for input".into()))?;

            // Derive the correct private key for this UTXO's address index
            let private_key = self.derive_private_key_for_index(mnemonic, utxo.address_index)?;
            let public_key = PublicKey::from_private_key(&secp, &private_key);
            let script_pubkey = bitcoin::Address::p2wpkh(&public_key.try_into().unwrap(), Network::Signet).script_pubkey();

            // Create signature for this input
            let mut sighash_cache = SighashCache::new(&tx);
            
            let sighash = sighash_cache
                .p2wpkh_signature_hash(
                    input_index,
                    &script_pubkey,
                    bitcoin::Amount::from_sat(utxo.amount_sats),
                    EcdsaSighashType::All,
                )
                .map_err(|e| crate::error::WalletError::Bitcoin(e.to_string()))?;

            let message = Message::from_digest(sighash.to_byte_array());
            let signature = secp.sign_ecdsa(&message, &private_key.inner);

            let mut sig_with_hashtype = signature.serialize_der().to_vec();
            sig_with_hashtype.push(EcdsaSighashType::All.to_u32() as u8);

            // Add witness data to the input
            signed_tx.input[input_index].witness.push(sig_with_hashtype);
            signed_tx.input[input_index].witness.push(public_key.to_bytes());
        }

        Ok(signed_tx)
    }
    
    /// Generate RGB invoice for receiving assets
    pub async fn generate_rgb_invoice(
        &self,
        wallet_name: &str,
        request: GenerateInvoiceRequest,
    ) -> Result<GenerateInvoiceResult, crate::error::WalletError> {
        // Verify wallet exists
        if !self.storage.wallet_exists(wallet_name) {
            return Err(crate::error::WalletError::WalletNotFound(wallet_name.to_string()));
        }
        
        // Parse contract ID
        use rgb::ContractId;
        let contract_id = ContractId::from_str(&request.contract_id)
            .map_err(|e| crate::error::WalletError::InvalidInput(format!("Invalid contract ID: {}", e)))?;
        
        // Initialize RGB Runtime (try without sync first for speed)
        let mut runtime = self.get_runtime_no_sync(wallet_name)?;
        
        // Generate auth token (blinded seal) from available UTXO
        let nonce = 0u64;  // Default nonce
        let auth = match runtime.auth_token(Some(nonce)) {
            Some(token) => token,
            None => {
                // No UTXOs available, need to sync with blockchain
                // This happens on first run or if UTXOs changed
                // Using 1 confirmation instead of 32 for faster response on signet/testnet
                // TODO: For mainnet, consider 3-6 confirmations for security
                runtime.update(1)
                    .map_err(|e| crate::error::WalletError::Rgb(format!("Sync failed: {:?}", e)))?;
                
                // Try again after sync
                runtime.auth_token(Some(nonce))
                    .ok_or_else(|| crate::error::WalletError::Rgb(
                        "No unspent outputs available for seal even after sync. Please create a UTXO first.".to_string()
                    ))?
            }
        };
        
        // Use native RGB invoice API with uri feature
        use rgb_invoice::{RgbInvoice, RgbBeneficiary};
        use hypersonic::Consensus;
        use strict_types::StrictVal;
        
        // Create beneficiary from auth token
        let beneficiary = RgbBeneficiary::Token(auth);
        
        // Create amount as StrictVal if provided
        let amount_val = request.amount.map(StrictVal::num);
        
        // Create invoice using RGB native API
        let invoice = RgbInvoice::new(
            contract_id,
            Consensus::Bitcoin,
            true, // testnet = true for signet
            beneficiary,
            amount_val,
        );
        
        // Serialize to URI string using native Display implementation
        let invoice_str = invoice.to_string();
        
        // Extract seal outpoint for display (auth token encodes the UTXO)
        let seal_outpoint = format!("{}", auth);
        
        Ok(GenerateInvoiceResult {
            invoice: invoice_str,
            contract_id: request.contract_id,
            amount: request.amount,
            seal_utxo: seal_outpoint,
        })
    }

    pub fn send_transfer(
        &self,
        wallet_name: &str,
        invoice_str: &str,
        fee_rate_sat_vb: Option<u64>,
    ) -> Result<SendTransferResponse, crate::error::WalletError> {
        use bpstd::psbt::{TxParams, PsbtConstructor};
        use bpstd::Sats;
        use rgb_invoice::RgbInvoice;
        use rgbp::CoinselectStrategy;
        
        // Parse invoice using native RGB uri feature
        let invoice = RgbInvoice::<rgb::ContractId>::from_str(invoice_str)
            .map_err(|e| crate::error::WalletError::InvalidInput(format!("Invalid invoice: {}", e)))?;
        
        // Initialize RGB runtime
        let mut runtime = self.get_runtime(wallet_name)?;
        
        // Set fee rate (default 1 sat/vB if not provided)
        let fee_sats = fee_rate_sat_vb.unwrap_or(1) * 250; // Rough estimate for typical RGB tx size
        let tx_params = TxParams::with(Sats::from(fee_sats));
        
        // Use aggregate coinselect strategy (same as RGB CLI default)
        let strategy = CoinselectStrategy::Aggregate;
        
        // Pay invoice - this returns PSBT and Payment
        // Note: pay_invoice internally handles DBC commit
        let (mut psbt, payment) = runtime.pay_invoice(&invoice, strategy, tx_params, None)
            .map_err(|e| crate::error::WalletError::Rgb(format!("Failed to create payment: {:?}", e)))?;
        
        // Extract contract ID from invoice scope
        let contract_id = invoice.scope;
        
        // Generate consignment BEFORE signing
        let consignment_dir = self.storage.base_dir().join("consignments");
        std::fs::create_dir_all(&consignment_dir)
            .map_err(|e| crate::error::WalletError::Internal(format!("Failed to create consignments dir: {}", e)))?;
        
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let consignment_filename = format!("transfer_{}_{}.rgbc", contract_id, timestamp);
        let consignment_path = consignment_dir.join(&consignment_filename);
        
        runtime.contracts
            .consign_to_file(&consignment_path, contract_id, payment.terminals)
            .map_err(|e| crate::error::WalletError::Rgb(format!("Failed to create consignment: {:?}", e)))?;
        
        // Sign the PSBT using our wallet signer
        let signer = self.create_signer(wallet_name)?;
        let signed_count = psbt.sign(&signer)
            .map_err(|e| crate::error::WalletError::Rgb(format!("Failed to approve signing: {:?}", e)))?;
        
        if signed_count == 0 {
            return Err(crate::error::WalletError::Rgb("Failed to sign any inputs".into()));
        }
        
        // Finalize the PSBT with wallet descriptor
        let finalized_count = psbt.finalize(runtime.wallet.descriptor());
        
        if finalized_count == 0 {
            return Err(crate::error::WalletError::Rgb("Failed to finalize any inputs".into()));
        }
        
        // Extract the signed transaction
        let bpstd_tx = psbt.extract()
            .map_err(|e| crate::error::WalletError::Rgb(format!("Failed to extract transaction: {} non-finalized inputs remain", e.0)))?;
        
        // Convert bpstd::Tx to hex string using :x format specifier
        // bpstd::Tx implements Display with :x formatting
        let tx_hex = format!("{:x}", bpstd_tx);
        
        // Get txid from bpstd::Tx
        let txid = bpstd_tx.txid().to_string();
        
        // Broadcast transaction
        self.broadcast_tx_hex(&tx_hex)?;
        
        // Return response
        Ok(SendTransferResponse {
            bitcoin_txid: txid,
            consignment_download_url: format!("/api/consignment/{}", consignment_filename),
            consignment_filename,
            status: "broadcasted".to_string(),
        })
    }

    pub(crate) fn create_signer(&self, wallet_name: &str) -> Result<WalletSigner, crate::error::WalletError> {
        let mnemonic = self.storage.load_mnemonic(wallet_name)?;
        Ok(WalletSigner::new(mnemonic, Network::Signet))
    }

    pub(crate) fn broadcast_tx_hex(&self, tx_hex: &str) -> Result<(), crate::error::WalletError> {
        let rt = tokio::runtime::Runtime::new()
            .map_err(|e| crate::error::WalletError::Network(format!("Runtime error: {}", e)))?;
        
        let result = rt.block_on(async {
            let client = reqwest::Client::new();
            let response = client
                .post("https://mempool.space/signet/api/tx")
                .body(tx_hex.to_string())
                .send()
                .await
                .map_err(|e| crate::error::WalletError::Network(e.to_string()))?;

            if !response.status().is_success() {
                let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
                return Err(crate::error::WalletError::Network(format!("Broadcast failed: {}", error_text)));
            }

            Ok(())
        });
        
        result
    }

    fn check_esplora_tx_status(&self, txid: &str) -> Result<bool, crate::error::WalletError> {
        let rt = tokio::runtime::Runtime::new()
            .map_err(|e| crate::error::WalletError::Network(format!("Runtime error: {}", e)))?;
        
        rt.block_on(async {
            let client = reqwest::Client::new();
            let url = format!("https://mempool.space/signet/api/tx/{}/status", txid);
            
            let response = client
                .get(&url)
                .send()
                .await
                .map_err(|e| crate::error::WalletError::Network(e.to_string()))?;

            if !response.status().is_success() {
                return Ok(false); // TX not found or error, assume not confirmed
            }

            let status: serde_json::Value = response
                .json()
                .await
                .map_err(|e| crate::error::WalletError::Network(e.to_string()))?;

            Ok(status["confirmed"].as_bool().unwrap_or(false))
        })
    }

    /// Accept a consignment (genesis or transfer) by importing it into RGB runtime.
    /// 
    /// This function validates and imports RGB consignments, automatically detecting:
    /// - Whether it's a genesis (initial contract) or transfer (token movement)
    /// - Bitcoin transaction ID for transfers
    /// - Transaction confirmation status (pending/confirmed/archived)
    /// 
    /// After import, users should sync their wallet to see updated token balances.
    pub fn accept_consignment(
        &self,
        wallet_name: &str,
        consignment_bytes: Vec<u8>,
    ) -> Result<crate::api::types::AcceptConsignmentResponse, crate::error::WalletError> {
        use std::io::Write;

        // 1. Save consignment to temp file
        let temp_dir = self.storage.base_dir().join("temp_consignments");
        std::fs::create_dir_all(&temp_dir)
            .map_err(|e| crate::error::WalletError::Internal(format!("Failed to create temp dir: {}", e)))?;

        let temp_filename = format!("accept_{}.rgbc", uuid::Uuid::new_v4());
        let temp_path = temp_dir.join(&temp_filename);
        
        let mut file = std::fs::File::create(&temp_path)
            .map_err(|e| crate::error::WalletError::Internal(format!("Failed to create temp file: {}", e)))?;
        file.write_all(&consignment_bytes)
            .map_err(|e| crate::error::WalletError::Internal(format!("Failed to write consignment: {}", e)))?;
        drop(file);

        // 2. Initialize runtime
        let mut runtime = self.get_runtime(wallet_name)?;

        // 3. Get contract IDs before importing
        let contract_ids_before: std::collections::HashSet<String> = runtime.contracts
            .contract_ids()
            .map(|id| id.to_string())
            .collect();

        // 4. Consume consignment (validates and imports)
        use std::convert::Infallible;
        runtime.consume_from_file(
            true,  // allow_unknown contracts
            &temp_path,
            |_, _, _| Result::<_, Infallible>::Ok(()),
        ).map_err(|e| crate::error::WalletError::Rgb(format!("Validation failed: {:?}", e)))?;

        // 5. Find new contract(s) that were imported
        let contract_ids_after: std::collections::HashSet<String> = runtime.contracts
            .contract_ids()
            .map(|id| id.to_string())
            .collect();
        
        let new_contracts: Vec<String> = contract_ids_after
            .difference(&contract_ids_before)
            .cloned()
            .collect();

        // Get first new contract as the imported one (usually only one)
        let contract_id_str = new_contracts.first()
            .ok_or_else(|| crate::error::WalletError::Rgb("No new contract found after import".into()))?
            .clone();

        // Parse contract ID for querying
        let contract_id = rgb::ContractId::from_str(&contract_id_str)
            .map_err(|e| crate::error::WalletError::Rgb(format!("Invalid contract ID: {:?}", e)))?;

        // 6. Query imported contract to determine type and extract witness info
        use rgb::WitnessStatus;

        let witness_count = runtime.contracts.contract_witness_count(contract_id);

        let (import_type, bitcoin_txid, status) = if witness_count == 0 {
            // Genesis: no witnesses (no Bitcoin TX)
            ("genesis".to_string(), None, "genesis_imported".to_string())
        } else {
            // Transfer: has witnesses (Bitcoin TXs)
            let witnesses: Vec<_> = runtime.contracts
                .contract_witnesses(contract_id)
                .collect();
            
            if let Some(last_witness) = witnesses.last() {
                // For TxoSeal, witness.id IS Txid
                let txid = last_witness.id.to_string();
                
                // Map witness status to our status string
                let status_str = match last_witness.status {
                    WitnessStatus::Genesis => "genesis_imported".to_string(),
                    WitnessStatus::Offchain => "offchain".to_string(),
                    WitnessStatus::Tentative => "pending".to_string(),
                    WitnessStatus::Mined(_) => "confirmed".to_string(),
                    WitnessStatus::Archived => "archived".to_string(),
                };
                
                ("transfer".to_string(), Some(txid), status_str)
            } else {
                // Fallback if witnesses iterator is empty despite count > 0
                ("transfer".to_string(), None, "imported".to_string())
            }
        };

        // 7. Cleanup temp file
        let _ = std::fs::remove_file(&temp_path);

        Ok(crate::api::types::AcceptConsignmentResponse {
            contract_id: contract_id_str,
            status,
            import_type,
            bitcoin_txid,
        })
    }

    /// Export a genesis consignment for syncing contract state across devices.
    /// 
    /// This allows users to share contract knowledge (not ownership transfer) 
    /// with the same wallet on different devices. No Bitcoin transaction is required.
    pub fn export_genesis_consignment(
        &self,
        wallet_name: &str,
        contract_id_str: &str,
    ) -> Result<crate::api::types::ExportGenesisResponse, crate::error::WalletError> {
        use std::str::FromStr;

        // 1. Parse contract ID
        let contract_id = rgb::ContractId::from_str(contract_id_str)
            .map_err(|e| crate::error::WalletError::Rgb(format!("Invalid contract ID: {:?}", e)))?;

        // 2. Initialize runtime to access contracts
        let runtime = self.get_runtime(wallet_name)?;

        // 3. Verify we have this contract
        if !runtime.contracts.has_contract(contract_id) {
            return Err(crate::error::WalletError::Rgb(
                format!("Contract {} not found in wallet", contract_id)
            ));
        }

        // 4. Get contract state to verify we have allocations
        let state = runtime.contracts.contract_state(contract_id);

        // Check if we have any owned states (just for validation)
        let has_allocations = state.owned.values().any(|states| !states.is_empty());
        
        if !has_allocations {
            return Err(crate::error::WalletError::Rgb(
                "No allocations found for contract".to_string()
            ));
        }

        // 5. Create consignment directory
        let consignment_filename = format!("genesis_{}.rgbc", contract_id);
        let exports_dir = self.storage.base_dir().join("exports");
        
        std::fs::create_dir_all(&exports_dir)
            .map_err(|e| crate::error::WalletError::Internal(
                format!("Failed to create exports directory: {}", e)
            ))?;

        let consignment_path = exports_dir.join(&consignment_filename);

        // 6. Export genesis consignment (empty terminals for genesis-only export)
        // For genesis consignment, we're exporting the contract state without
        // transferring to new seals - the recipient will already own the UTXOs
        let empty_terminals: Vec<rgb::AuthToken> = Vec::new();
        runtime.contracts
            .consign_to_file(&consignment_path, contract_id, empty_terminals)
            .map_err(|e| crate::error::WalletError::Rgb(
                format!("Failed to create genesis consignment: {:?}", e)
            ))?;

        // 7. Get file size
        let file_size = std::fs::metadata(&consignment_path)
            .map_err(|e| crate::error::WalletError::Internal(
                format!("Failed to read file metadata: {}", e)
            ))?
            .len();

        Ok(crate::api::types::ExportGenesisResponse {
            contract_id: contract_id_str.to_string(),
            consignment_filename: consignment_filename.clone(),
            file_size_bytes: file_size,
            download_url: format!("/api/genesis/{}", consignment_filename),
        })
    }
}

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletInfo {
    pub name: String,
    pub mnemonic: String,
    pub first_address: String,
    pub descriptor: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletMetadata {
    pub name: String,
    pub created_at: String,
    pub last_synced: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddressInfo {
    pub index: u32,
    pub address: String,
    pub used: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncResult {
    pub synced_height: u64,
    pub addresses_checked: u32,
    pub new_transactions: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NextAddressInfo {
    pub address: String,
    pub index: u32,
    pub total_used: usize,
    pub descriptor: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateUtxoRequest {
    pub amount_btc: Option<f64>,
    pub fee_rate_sat_vb: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateUtxoResult {
    pub txid: String,
    pub amount_sats: u64,
    pub fee_sats: u64,
    pub target_address: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnlockUtxoRequest {
    pub txid: String,
    pub vout: u32,
    pub fee_rate_sat_vb: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnlockUtxoResult {
    pub txid: String,
    pub recovered_sats: u64,
    pub fee_sats: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerateInvoiceRequest {
    pub contract_id: String,
    pub amount: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerateInvoiceResult {
    pub invoice: String,
    pub contract_id: String,
    pub amount: Option<u64>,
    pub seal_utxo: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendTransferResponse {
    pub bitcoin_txid: String,
    pub consignment_download_url: String,
    pub consignment_filename: String,
    pub status: String,
}
