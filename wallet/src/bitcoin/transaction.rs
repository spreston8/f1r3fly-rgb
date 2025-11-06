use bitcoin::blockdata::script::ScriptBuf;
use bitcoin::blockdata::transaction::{Transaction, TxIn, TxOut};
use bitcoin::blockdata::witness::Witness;
use bitcoin::hashes::Hash;
use bitcoin::secp256k1::{Message, Secp256k1};
use bitcoin::sighash::{EcdsaSighashType, SighashCache};
use bitcoin::transaction::{OutPoint, Sequence};
use bitcoin::{absolute, Address, Network};
use bitcoin::{PrivateKey, PublicKey};

use crate::bitcoin::balance_checker::UTXO;

pub struct TransactionBuilder {
    network: Network,
}

impl TransactionBuilder {
    /// Create a new transaction builder for the specified network
    pub fn new(network: Network) -> Self {
        Self { network }
    }

    /// Build a transaction that sends funds to a recipient address with change back to the same address
    pub fn build_send_to_self(
        &self,
        available_utxos: &[UTXO],
        target_amount_sats: u64,
        fee_rate_sat_vb: u64,
        recipient_address: Address,
    ) -> Result<Transaction, crate::error::WalletError> {
        let selected_utxos =
            self.select_utxos(available_utxos, target_amount_sats, fee_rate_sat_vb)?;

        let total_input: u64 = selected_utxos.iter().map(|u| u.amount_sats).sum();

        let estimated_size = self.estimate_tx_size(selected_utxos.len(), 2);
        let fee = estimated_size * fee_rate_sat_vb;

        if total_input < target_amount_sats + fee {
            return Err(crate::error::WalletError::InsufficientFunds(format!(
                "Need {} sats (amount + fee), but only have {} sats",
                target_amount_sats + fee,
                total_input
            )));
        }

        let change_amount = total_input - target_amount_sats - fee;

        let mut tx = Transaction {
            version: bitcoin::transaction::Version::TWO,
            lock_time: absolute::LockTime::ZERO,
            input: vec![],
            output: vec![],
        };

        for utxo in selected_utxos {
            tx.input.push(TxIn {
                previous_output: OutPoint {
                    txid: utxo.txid.parse().map_err(|e| {
                        crate::error::WalletError::Bitcoin(format!("Invalid txid: {}", e))
                    })?,
                    vout: utxo.vout,
                },
                script_sig: ScriptBuf::new(),
                sequence: Sequence::MAX,
                witness: Witness::new(),
            });
        }

        tx.output.push(TxOut {
            value: bitcoin::Amount::from_sat(target_amount_sats),
            script_pubkey: recipient_address.script_pubkey(),
        });

        if change_amount >= 546 {
            tx.output.push(TxOut {
                value: bitcoin::Amount::from_sat(change_amount),
                script_pubkey: recipient_address.script_pubkey(),
            });
        }

        Ok(tx)
    }

    /// Build a transaction that sends funds to a recipient with change to a specified address
    pub fn build_send_tx(
        &self,
        utxos: &[UTXO],
        to_address: Address,
        amount_sats: u64,
        change_address: Address,
        fee_rate_sat_vb: u64,
    ) -> Result<Transaction, crate::error::WalletError> {
        let total_input: u64 = utxos.iter().map(|u| u.amount_sats).sum();
        let estimated_size = self.estimate_tx_size(utxos.len(), 2);
        let fee = estimated_size * fee_rate_sat_vb;

        if total_input < amount_sats + fee {
            return Err(crate::error::WalletError::InsufficientFunds(format!(
                "Need {} sats (amount + fee), but only have {} sats",
                amount_sats + fee,
                total_input
            )));
        }

        let change_amount = total_input - amount_sats - fee;

        let mut tx = Transaction {
            version: bitcoin::transaction::Version::TWO,
            lock_time: absolute::LockTime::ZERO,
            input: vec![],
            output: vec![],
        };

        for utxo in utxos {
            tx.input.push(TxIn {
                previous_output: OutPoint {
                    txid: utxo.txid.parse().map_err(|e| {
                        crate::error::WalletError::Bitcoin(format!("Invalid txid: {}", e))
                    })?,
                    vout: utxo.vout,
                },
                script_sig: ScriptBuf::new(),
                sequence: Sequence::MAX,
                witness: Witness::new(),
            });
        }

        tx.output.push(TxOut {
            value: bitcoin::Amount::from_sat(amount_sats),
            script_pubkey: to_address.script_pubkey(),
        });

        // Add change output if above dust limit
        if change_amount >= 546 {
            tx.output.push(TxOut {
                value: bitcoin::Amount::from_sat(change_amount),
                script_pubkey: change_address.script_pubkey(),
            });
        }

        Ok(tx)
    }

    /// Build a transaction that unlocks a single UTXO and sends all funds (minus fee) to a destination
    pub fn build_unlock_utxo_tx(
        &self,
        utxo: &UTXO,
        destination_address: Address,
        fee_rate_sat_vb: u64,
    ) -> Result<Transaction, crate::error::WalletError> {
        let estimated_size = self.estimate_tx_size(1, 1);
        let fee = estimated_size * fee_rate_sat_vb;

        if utxo.amount_sats <= fee {
            return Err(crate::error::WalletError::InsufficientFunds(format!(
                "UTXO amount ({} sats) is not enough to cover fee ({} sats)",
                utxo.amount_sats, fee
            )));
        }

        let output_amount = utxo.amount_sats - fee;

        if output_amount < 546 {
            return Err(crate::error::WalletError::InsufficientFunds(format!(
                "Output amount ({} sats) would be below dust limit (546 sats)",
                output_amount
            )));
        }

        let mut tx = Transaction {
            version: bitcoin::transaction::Version::TWO,
            lock_time: absolute::LockTime::ZERO,
            input: vec![],
            output: vec![],
        };

        tx.input.push(TxIn {
            previous_output: OutPoint {
                txid: utxo.txid.parse().map_err(|e| {
                    crate::error::WalletError::Bitcoin(format!("Invalid txid: {}", e))
                })?,
                vout: utxo.vout,
            },
            script_sig: ScriptBuf::new(),
            sequence: Sequence::MAX,
            witness: Witness::new(),
        });

        tx.output.push(TxOut {
            value: bitcoin::Amount::from_sat(output_amount),
            script_pubkey: destination_address.script_pubkey(),
        });

        Ok(tx)
    }

    /// Sign a transaction using a single private key for all inputs
    pub fn sign_transaction(
        &self,
        mut tx: Transaction,
        utxos: &[UTXO],
        private_key: &PrivateKey,
    ) -> Result<Transaction, crate::error::WalletError> {
        let secp = Secp256k1::new();
        let public_key = PublicKey::from_private_key(&secp, private_key);
        let script_pubkey =
            Address::p2wpkh(&public_key.try_into().unwrap(), self.network).script_pubkey();

        let mut signatures = Vec::new();

        for (input_index, input) in tx.input.iter().enumerate() {
            let utxo = utxos
                .iter()
                .find(|u| {
                    if let Ok(txid) = u.txid.parse::<bitcoin::Txid>() {
                        txid == input.previous_output.txid && u.vout == input.previous_output.vout
                    } else {
                        false
                    }
                })
                .ok_or_else(|| {
                    crate::error::WalletError::Bitcoin("UTXO not found for input".into())
                })?;

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

            signatures.push((sig_with_hashtype, public_key.to_bytes()));
        }

        for (input, (sig, pubkey)) in tx.input.iter_mut().zip(signatures.iter()) {
            input.witness.push(sig.clone());
            input.witness.push(pubkey.to_vec());
        }

        Ok(tx)
    }

    /// Select UTXOs to cover the target amount plus fees using a largest-first strategy
    fn select_utxos(
        &self,
        available_utxos: &[UTXO],
        target_amount: u64,
        fee_rate: u64,
    ) -> Result<Vec<UTXO>, crate::error::WalletError> {
        let mut sorted_utxos = available_utxos.to_vec();
        sorted_utxos.sort_by(|a, b| b.amount_sats.cmp(&a.amount_sats));

        let mut selected = Vec::new();
        let mut total = 0u64;

        for utxo in sorted_utxos {
            selected.push(utxo.clone());
            total += utxo.amount_sats;

            let estimated_size = self.estimate_tx_size(selected.len(), 2);
            let estimated_fee = estimated_size * fee_rate;

            if total >= target_amount + estimated_fee + 546 {
                return Ok(selected);
            }
        }

        Err(crate::error::WalletError::InsufficientFunds(format!(
            "Cannot create UTXO of {} sats with available balance",
            target_amount
        )))
    }

    /// Estimate transaction size in virtual bytes based on number of inputs and outputs
    fn estimate_tx_size(&self, num_inputs: usize, num_outputs: usize) -> u64 {
        let base_size = 10;
        let input_size = 68;
        let output_size = 34;

        (base_size + (num_inputs * input_size) + (num_outputs * output_size)) as u64
    }
}

/// Broadcast a signed transaction to the network and return the transaction ID
pub async fn broadcast_transaction(
    tx: &Transaction,
    network: Network,
) -> Result<String, crate::error::WalletError> {
    let tx_hex = bitcoin::consensus::encode::serialize_hex(tx);

    // Use configured Esplora URL for Regtest, otherwise use mempool.space
    let base_url = if matches!(network, Network::Regtest) {
        let config = crate::config::WalletConfig::from_env();
        config.esplora_url
    } else {
        match network {
            Network::Signet => "https://mempool.space/signet/api".to_string(),
            Network::Testnet => "https://mempool.space/testnet/api".to_string(),
            _ => "https://mempool.space/api".to_string(),
        }
    };

    log::debug!("Broadcasting transaction to: {}/tx", base_url);

    let client = reqwest::Client::new();
    let response = client
        .post(format!("{}/tx", base_url))
        .body(tx_hex)
        .send()
        .await
        .map_err(|e| crate::error::WalletError::Network(e.to_string()))?;

    if !response.status().is_success() {
        let error_text = response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string());
        return Err(crate::error::WalletError::Network(format!(
            "Broadcast failed: {}",
            error_text
        )));
    }

    let txid = response
        .text()
        .await
        .map_err(|e| crate::error::WalletError::Network(e.to_string()))?;

    Ok(txid)
}

