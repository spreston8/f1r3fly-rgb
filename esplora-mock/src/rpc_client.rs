/// Bitcoin Core RPC client wrapper
/// 
/// Provides a clean interface for querying Bitcoin Core in Regtest mode.

use anyhow::{Context, Result};
use bitcoincore_rpc::bitcoin::{Address, BlockHash, Txid};
use bitcoincore_rpc::{Auth, Client, RpcApi};
use std::str::FromStr;

use crate::types::*;

pub struct BitcoinRpcClient {
    client: Client,
}

impl BitcoinRpcClient {
    /// Create a new Bitcoin RPC client from URL and authentication
    pub fn new(url: String, username: String, password: String) -> Result<Self> {
        let auth = Auth::UserPass(username, password);
        let client = Client::new(&url, auth)
            .context("Failed to create Bitcoin RPC client")?;
        
        // Test connection
        client.get_blockchain_info()
            .context("Failed to connect to Bitcoin Core - is it running?")?;
        
        log::info!("Connected to Bitcoin Core at {}", url);
        
        Ok(Self { client })
    }

    /// Get current blockchain height
    pub fn get_block_count(&self) -> Result<u64> {
        Ok(self.client.get_block_count()?)
    }

    /// Get current blockchain tip hash
    pub fn get_best_block_hash(&self) -> Result<String> {
        Ok(self.client.get_best_block_hash()?.to_string())
    }

    /// Broadcast a raw transaction
    pub fn send_raw_transaction(&self, tx_hex: &str) -> Result<String> {
        // bitcoincore-rpc accepts hex string directly
        let txid = self.client.send_raw_transaction(tx_hex)?;
        Ok(txid.to_string())
    }

    /// Get transaction details with confirmation status
    pub fn get_transaction(&self, txid: &str) -> Result<TxResponse> {
        let txid = Txid::from_str(txid)
            .context("Invalid txid")?;
        
        let tx_result = self.client.get_raw_transaction_info(&txid, None)?;
        
        let status = if let Some(confirmations) = tx_result.confirmations {
            let block_hash = tx_result.blockhash.map(|h| h.to_string());
            let block_height = if let Some(ref hash_str) = block_hash {
                self.get_block_height_from_hash(hash_str).ok()
            } else {
                None
            };
            
            TxStatusResponse {
                confirmed: confirmations > 0,
                block_height,
                block_hash,
                block_time: tx_result.blocktime.map(|t| t as u64),
            }
        } else {
            TxStatusResponse {
                confirmed: false,
                block_height: None,
                block_hash: None,
                block_time: None,
            }
        };

        // Parse inputs
        let vin: Vec<TxInput> = tx_result.vin.iter().map(|input| {
            TxInput {
                txid: input.txid.map(|t| t.to_string()).unwrap_or_default(),
                vout: input.vout.unwrap_or(0),
                // Prevout intentionally omitted - would require additional RPC call per input
                // Not needed for RGB wallet operations (only witness data matters)
                prevout: None,
                scriptsig: input.script_sig.as_ref().map(|s| hex::encode(&s.hex)).unwrap_or_default(),
                scriptsig_asm: input.script_sig.as_ref().map(|s| s.asm.clone()).unwrap_or_default(),
                witness: input.txinwitness.as_ref().map(|w| w.iter().map(hex::encode).collect()),
                is_coinbase: input.txid.is_none(),
                sequence: input.sequence,
            }
        }).collect();

        // Parse outputs
        let vout: Vec<TxOutput> = tx_result.vout.iter().map(|output| {
            let address = output.script_pub_key.addresses
                .first()
                .map(|a| a.clone().assume_checked().to_string());
            
            TxOutput {
                scriptpubkey: hex::encode(&output.script_pub_key.hex),
                scriptpubkey_asm: output.script_pub_key.asm.clone(),
                scriptpubkey_type: output.script_pub_key.type_.as_ref().map(|t| format!("{:?}", t)).unwrap_or_default(),
                scriptpubkey_address: address,
                value: output.value.to_sat(),
            }
        }).collect();

        Ok(TxResponse {
            txid: tx_result.txid.to_string(),
            version: tx_result.version as i32,
            locktime: tx_result.locktime,
            vin,
            vout,
            size: tx_result.size,
            weight: tx_result.size * 4, // Estimate weight from size
            // Fee calculation requires fetching all input amounts - not needed for RGB operations
            // RGB only requires confirmation status and witness data
            fee: 0,
            status,
        })
    }

    /// Get transaction status (lightweight version)
    pub fn get_transaction_status(&self, txid: &str) -> Result<TxStatusResponse> {
        let txid = Txid::from_str(txid)
            .context("Invalid txid")?;
        
        let tx_result = self.client.get_raw_transaction_info(&txid, None)?;
        
        if let Some(confirmations) = tx_result.confirmations {
            let block_hash = tx_result.blockhash.map(|h| h.to_string());
            let block_height = if let Some(ref hash_str) = block_hash {
                self.get_block_height_from_hash(hash_str).ok()
            } else {
                None
            };
            
            Ok(TxStatusResponse {
                confirmed: confirmations > 0,
                block_height,
                block_hash,
                block_time: tx_result.blocktime.map(|t| t as u64),
            })
        } else {
            Ok(TxStatusResponse {
                confirmed: false,
                block_height: None,
                block_hash: None,
                block_time: None,
            })
        }
    }

    /// Get raw transaction hex
    pub fn get_raw_transaction_hex(&self, txid: &str) -> Result<String> {
        let txid = Txid::from_str(txid)
            .context("Invalid txid")?;
        
        // Get raw transaction info which includes hex
        let tx_info = self.client.get_raw_transaction_info(&txid, None)?;
        Ok(hex::encode(&tx_info.hex))
    }

    /// Get UTXOs for an address
    pub fn get_address_utxos(&self, address: &str) -> Result<Vec<UtxoResponse>> {
        let address = Address::from_str(address)
            .context("Invalid address")?
            .assume_checked();
        
        // Import address if not already imported (idempotent)
        let _ = self.client.import_address(&address, None, Some(false));
        
        // List unspent for this address
        let unspent = self.client.list_unspent(
            Some(0),  // Min confirmations (include mempool)
            None,     // Max confirmations
            Some(&[&address]),
            None,
            None,
        )?;

        let current_height = self.get_block_count()?;

        let utxos: Vec<UtxoResponse> = unspent.into_iter().map(|utxo| {
            let (confirmed, block_height, block_hash, block_time) = if utxo.confirmations > 0 {
                let height = current_height.saturating_sub(utxo.confirmations as u64 - 1);
                // Get block hash for this height
                let block_hash = self.client.get_block_hash(height)
                    .ok()
                    .map(|h| h.to_string());
                let block_time = if let Some(ref hash_str) = block_hash {
                    self.get_block_time_from_hash(hash_str).ok()
                } else {
                    None
                };
                (true, Some(height), block_hash, block_time)
            } else {
                (false, None, None, None)
            };

            UtxoResponse {
                txid: utxo.txid.to_string(),
                vout: utxo.vout,
                value: utxo.amount.to_sat(),
                status: UtxoStatus {
                    confirmed,
                    block_height,
                    block_hash,
                    block_time,
                },
            }
        }).collect();

        Ok(utxos)
    }

    /// Check if an output is spent
    pub fn get_output_spend_status(&self, txid: &str, vout: u32) -> Result<OutputSpendStatus> {
        let txid_obj = Txid::from_str(txid)
            .context("Invalid txid")?;
        
        // Bitcoin Core's get_tx_out returns None if output is spent or doesn't exist
        let tx_out = self.client.get_tx_out(&txid_obj, vout, Some(false))?;
        
        if tx_out.is_none() {
            // Output is spent or doesn't exist
            // Note: Finding the spending transaction requires a txindex or specialized indexer
            // Bitcoin Core RPC doesn't provide this without full transaction indexing
            // For RGB wallet operations, knowing spent=true is sufficient
            Ok(OutputSpendStatus {
                spent: true,
                txid: None,  // Spending txid not available without txindex
                vin: None,   // Spending input index not available
                status: None,
            })
        } else {
            // Output is unspent
            Ok(OutputSpendStatus {
                spent: false,
                txid: None,
                vin: None,
                status: None,
            })
        }
    }

    /// Mine blocks (Regtest only)
    pub fn generate_blocks(&self, count: u64, address_str: Option<String>) -> Result<Vec<String>> {
        let address = if let Some(addr) = address_str {
            Address::from_str(&addr)
                .context("Invalid address")?
                .assume_checked()
        } else {
            // Generate to a wallet address
            self.client.get_new_address(None, None)?
                .assume_checked()
        };

        let block_hashes = self.client.generate_to_address(count, &address)?;
        Ok(block_hashes.iter().map(|h| h.to_string()).collect())
    }

    /// Helper: Get block height from hash
    fn get_block_height_from_hash(&self, hash: &str) -> Result<u64> {
        let block_hash = BlockHash::from_str(hash)?;
        let block = self.client.get_block_info(&block_hash)?;
        Ok(block.height as u64)
    }

    /// Helper: Get block time from hash
    fn get_block_time_from_hash(&self, hash: &str) -> Result<u64> {
        let block_hash = BlockHash::from_str(hash)?;
        let block = self.client.get_block_header_info(&block_hash)?;
        Ok(block.time as u64)
    }
}

