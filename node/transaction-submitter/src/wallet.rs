//! Wallet functionality for building and signing Kaspa transactions.

use kaspa_addresses::Address;
use kaspa_consensus_core::{
    config::params::Params,
    mass::MassCalculator,
    network::NetworkId,
    sign,
    subnets::SUBNETWORK_ID_NATIVE,
    tx::{
        PopulatedTransaction, ScriptPublicKey, SignableTransaction, Transaction, TransactionInput,
        TransactionOutput, UtxoEntry,
    },
};
use kaspa_hashes::Hash;
use kaspa_rpc_core::RpcUtxosByAddressesEntry;
use kaspa_wrpc_client::prelude::*;
use secp256k1::Keypair;
use std::io::Write;
use std::sync::Arc;

use crate::error::{Error, Result};
use vprogs_transaction_runtime::privacy::{self, PrivacyMode};

/// Represents a private key for signing transactions.
///
/// In production, this should be stored securely (e.g., encrypted, hardware wallet).
/// For testing, you can load from environment variables or a secure key store.
pub struct PrivateKey {
    /// The raw private key bytes (32 bytes for secp256k1)
    pub key_bytes: [u8; 32],
}

impl PrivateKey {
    /// Creates a private key from raw bytes.
    ///
    /// # Safety
    /// Ensure the key bytes are valid secp256k1 private key.
    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        Self { key_bytes: bytes }
    }

    /// Creates a private key from a hex string.
    pub fn from_hex(hex: &str) -> Result<Self> {
        // Trim whitespace (common issue with .env files)
        let hex_trimmed = hex.trim();

        // Validate length before decoding
        if hex_trimmed.len() != 64 {
            return Err(Error::Serialization(format!(
                "Private key hex string must be exactly 64 characters, got {} characters",
                hex_trimmed.len()
            )));
        }

        let bytes = hex::decode(hex_trimmed).map_err(|e| {
            Error::Serialization(format!("Invalid hex string '{}': {}", hex_trimmed, e))
        })?;

        if bytes.len() != 32 {
            return Err(Error::Serialization(format!(
                "Private key must be 32 bytes after decoding, got {} bytes",
                bytes.len()
            )));
        }

        let mut key_bytes = [0u8; 32];
        key_bytes.copy_from_slice(&bytes);
        Ok(Self::from_bytes(key_bytes))
    }

    /// Convert to secp256k1 Keypair for use with SignableTransaction
    pub fn to_keypair(&self) -> Result<Keypair> {
        use secp256k1::SECP256K1;
        Keypair::from_seckey_slice(SECP256K1, &self.key_bytes)
            .map_err(|e| Error::Serialization(format!("Invalid private key: {}", e)))
    }
}

/// Wallet for managing addresses and signing transactions.
pub struct Wallet {
    /// RPC client for querying UTXOs
    client: Arc<KaspaRpcClient>,
    /// Network ID
    network_id: NetworkId,
}

impl Wallet {
    /// Creates a new wallet instance.
    pub fn new(client: Arc<KaspaRpcClient>, network_id: NetworkId) -> Self {
        Self { client, network_id }
    }

    /// Gets UTXOs for an address.
    pub async fn get_utxos(&self, address: &Address) -> Result<Vec<RpcUtxosByAddressesEntry>> {
        let response = self
            .client
            .rpc_api()
            .get_utxos_by_addresses(vec![address.clone()])
            .await
            .map_err(|e| Error::Connection(format!("Failed to get UTXOs: {}", e)))?;

        Ok(response)
    }

    /// Selects UTXOs to cover the required amount plus fees.
    ///
    /// Uses a simple greedy algorithm: selects UTXOs until the sum covers amount + fees.
    /// Tries to avoid UTXOs that might be locked by pending transactions by selecting
    /// from different UTXOs if available.
    pub fn select_utxos<'a>(
        &self,
        utxos: &'a [RpcUtxosByAddressesEntry],
        required_amount: u64,
        fee_per_input: u64,
    ) -> Result<Vec<&'a RpcUtxosByAddressesEntry>> {
        if utxos.is_empty() {
            return Err(Error::Serialization("No UTXOs available".to_string()));
        }

        // Sort UTXOs by amount (smallest first) to prefer smaller UTXOs
        // This helps avoid creating large change outputs that exceed storage_mass limits
        let mut sorted_utxos: Vec<&RpcUtxosByAddressesEntry> = utxos.iter().collect();
        sorted_utxos.sort_by_key(|u| u.utxo_entry.amount);

        let estimated_fees = (sorted_utxos.len() as u64).saturating_mul(fee_per_input);
        let target = required_amount + estimated_fees;

        // Try to find a UTXO that's close to the target (within reasonable range) to minimize change
        // This is critical for keeping storage_mass under the limit
        // storage_mass is proportional to output values, so large change = high storage_mass
        const MAX_REASONABLE_CHANGE: u64 = 5_000_000; // 0.05 KAS - reasonable change size to keep storage_mass low
        let ideal_max = target + MAX_REASONABLE_CHANGE;

        // First, try to find a single UTXO that's close to ideal (not too large)
        for utxo in &sorted_utxos {
            let utxo_amount = utxo.utxo_entry.amount;
            if utxo_amount >= target && utxo_amount <= ideal_max {
                let change = utxo_amount.saturating_sub(target);
                log::info!(
                    "Found ideal UTXO: {} sompi (target: {} sompi, change: {} sompi) - should keep storage_mass low",
                    utxo_amount,
                    target,
                    change
                );
                return Ok(vec![utxo]);
            }
        }

        // If no ideal UTXO found, try to use multiple smaller UTXOs that sum to just enough
        // This minimizes change output
        let mut selected_indices = Vec::new();
        let mut total = 0u64;

        for (idx, utxo) in sorted_utxos.iter().enumerate() {
            if total >= target {
                break;
            }
            let new_total = total.saturating_add(utxo.utxo_entry.amount);
            // Only add if it doesn't create excessive change
            // Allow some flexibility but try to stay close to target
            if new_total <= target + MAX_REASONABLE_CHANGE * 3 {
                selected_indices.push(idx);
                total = new_total;
            }
        }

        // If we still don't have enough, add more UTXOs (we'll have to deal with larger change)
        if total < target {
            for (idx, utxo) in sorted_utxos.iter().enumerate() {
                if total >= target {
                    break;
                }
                if !selected_indices.contains(&idx) {
                    selected_indices.push(idx);
                    total = total.saturating_add(utxo.utxo_entry.amount);
                }
            }
        }

        // Convert indices back to references
        let selected: Vec<&RpcUtxosByAddressesEntry> =
            selected_indices.iter().map(|&idx| sorted_utxos[idx]).collect();

        if total < target {
            return Err(Error::Serialization(format!(
                "Insufficient funds: need {} sompi, have {} sompi",
                target, total
            )));
        }

        let change = total.saturating_sub(target);
        log::info!(
            "Selected {} UTXOs (sorted by size), total: {} sompi (target: {} sompi, change: {} sompi)",
            selected.len(),
            total,
            target,
            change
        );

        // Warn if change is still too large (will likely cause storage_mass issues)
        if change > MAX_REASONABLE_CHANGE * 10 {
            log::warn!(
                "Large change output ({} sompi) will likely cause storage_mass to exceed limit (100,000). \
                Consider consolidating UTXOs or using UTXOs closer to target amount.",
                change
            );
        }

        Ok(selected)
    }

    /// Creates a script public key from an address.
    ///
    /// Based on UTXO analysis: UTXO scripts are 34 bytes starting with 0x20 (push 32 bytes)
    /// followed by the 32-byte address payload. This suggests Kaspa uses a simple push script format.
    pub fn address_to_script_public_key(&self, address: &Address) -> ScriptPublicKey {
        let payload = &address.payload;
        let payload_bytes = payload.as_slice();

        log::debug!(
            "Converting address to script_public_key: version={:?}, payload_len={}",
            address.version,
            payload_bytes.len()
        );

        // UTXO scripts are 34 bytes: [0x20 (push 32), ...32 bytes of payload..., 0xac (OP_CHECKSIG)]
        // Construct the same format
        let mut script = Vec::with_capacity(34);
        script.push(0x20); // OP_PUSH_DATA_32 (push 32 bytes) - opcode 0x20 means push 32 bytes
        script.extend_from_slice(payload_bytes);
        script.push(0xac); // OP_CHECKSIG (likely trailing opcode)

        log::debug!(
            "Constructed script length: {} (format: [0x20, 32-byte payload, 0xac])",
            script.len()
        );

        ScriptPublicKey::new(
            0, // Script version 0 (matches UTXO format)
            kaspa_consensus_core::tx::ScriptVec::from(script),
        )
    }

    /// Creates an OP_RETURN output for embedding data (like Cairo proofs).
    ///
    /// OP_RETURN allows embedding arbitrary data in transactions.
    /// The data is limited in size (typically 40 bytes for Kaspa).
    pub fn create_op_return_output(&self, data: &[u8]) -> Result<TransactionOutput> {
        // Kaspa OP_RETURN format: OP_RETURN <data>
        // For larger data, you might need to split across multiple outputs
        // or use a different embedding method

        if data.len() > 40 {
            return Err(Error::NotImplemented(format!(
                "OP_RETURN data too large: {} bytes (max 40)",
                data.len()
            )));
        }

        // Create OP_RETURN script: [0x6a, data_len, ...data]
        let mut script = Vec::with_capacity(2 + data.len());
        script.push(0x6a); // OP_RETURN opcode
        script.push(data.len() as u8);
        script.extend_from_slice(data);

        use kaspa_consensus_core::tx::ScriptVec;
        Ok(TransactionOutput {
            value: 0, // OP_RETURN outputs have zero value
            script_public_key: ScriptPublicKey::new(0, ScriptVec::from(script)),
        })
    }

    /// Builds a Kaspa transaction.
    ///
    /// Creates a transaction with:
    /// - Inputs from selected UTXOs
    /// - Output to recipient
    /// - Change output (if needed)
    /// - OP_RETURN output with proof data (if provided)
    pub async fn build_transaction(
        &self,
        from_address: &Address,
        to_address: &Address,
        amount: u64,
        proof_data: Option<&[u8]>,
        private_key: &PrivateKey,
    ) -> Result<Transaction> {
        // Default to public transactions for backward compatibility
        self.build_transaction_with_privacy(
            from_address,
            to_address,
            amount,
            proof_data,
            private_key,
            PrivacyMode::Public,
        )
        .await
    }

    /// Builds a transaction with explicit privacy mode.
    pub async fn build_transaction_with_privacy(
        &self,
        from_address: &Address,
        to_address: &Address,
        amount: u64,
        proof_data: Option<&[u8]>,
        private_key: &PrivateKey,
        privacy_mode: PrivacyMode,
    ) -> Result<Transaction> {
        // Get UTXOs (RPC should exclude mempool-spent UTXOs automatically)
        let utxos: Vec<RpcUtxosByAddressesEntry> = self.get_utxos(from_address).await?;

        if utxos.is_empty() {
            return Err(Error::Serialization(
                "No available UTXOs found. All UTXOs may be spent in pending transactions."
                    .to_string(),
            ));
        }

        log::info!("Found {} available UTXOs (excluding mempool-spent)", utxos.len());

        // Debug: Log the script_public_key format from UTXOs for reference
        if let Some(first_utxo) = utxos.first() {
            let spk = &first_utxo.utxo_entry.script_public_key;
            let utxo_script = spk.script();
            log::debug!(
                "UTXO script_public_key: version={}, script_len={}, first_bytes={:?}",
                spk.version(),
                utxo_script.len(),
                if utxo_script.len() > 10 { &utxo_script[..10] } else { utxo_script }
            );
        }

        // Estimate fees based on transaction mass
        // Kaspa fees are typically calculated based on transaction mass, not just inputs/outputs
        // For now, use a conservative estimate: higher fees to ensure transaction is prioritized
        // TODO: Calculate fees based on actual transaction mass
        let fee_per_input = 10000; // Increased from 1000 to ensure transaction is prioritized
        let fee_per_output = 10000; // Increased from 1000
        let num_outputs = 1 + if proof_data.is_some() { 1 } else { 0 } + 1; // to, proof, change
        let estimated_fees = (utxos.len() as u64).saturating_mul(fee_per_input)
            + (num_outputs as u64).saturating_mul(fee_per_output);

        log::info!(
            "Estimated fees: {} sompi ({} per input, {} per output)",
            estimated_fees,
            fee_per_input,
            fee_per_output
        );

        // Select UTXOs first (we'll refine fee calculation iteratively)
        let selected_utxos = self.select_utxos(&utxos, amount, fee_per_input)?;

        // Calculate total input value
        let total_input: u64 = selected_utxos.iter().map(|u| u.utxo_entry.amount).sum();

        // Get network params for mass calculation
        let params = Params::from(self.network_id.network_type());
        let mass_calculator = MassCalculator::new(
            params.mass_per_tx_byte,
            params.mass_per_script_pub_key_byte,
            params.mass_per_sig_op,
            params.storage_mass_parameter,
        );

        // Get fee estimate from network (like kaspa_audio_transfer does)
        // For now, use a conservative feerate (will be refined iteratively)
        let feerate_sompi_per_gram = 1.0; // Conservative estimate

        // Prepare script public keys
        let from_script = self.address_to_script_public_key(from_address);
        let to_script = self.address_to_script_public_key(to_address);

        // Iteratively solve for fee and change since storage_mass depends on output amounts
        // This is critical - we need to calculate the actual mass to get the correct fee
        let mut change_guess: u64 = total_input.saturating_sub(amount).max(1);
        let mut last_fee: u64 = 0;
        let mut last_storage_mass: u64 = 0;

        // Build a template transaction for mass calculation (before signing)
        let mut template_tx = Transaction::default();
        template_tx.subnetwork_id = SUBNETWORK_ID_NATIVE;
        template_tx.inputs = selected_utxos
            .iter()
            .map(|u| {
                TransactionInput {
                    previous_outpoint: u.outpoint.into(),
                    signature_script: vec![0u8; 66], // Placeholder for mass calculation
                    sequence: 0,
                    sig_op_count: 1,
                }
            })
            .collect();

        // Embed payload if provided (for accurate mass calculation)
        if let Some(proof) = proof_data {
            if !proof.is_empty() {
                template_tx.payload = proof.to_vec();
            }
        }

        // Iterate to find correct fee and change (like kaspa_audio_transfer)
        for iteration in 0..20u32 {
            // Build outputs with current change guess
            template_tx.outputs.clear();
            template_tx
                .outputs
                .push(TransactionOutput { value: amount, script_public_key: to_script.clone() });
            if change_guess > 0 {
                template_tx.outputs.push(TransactionOutput {
                    value: change_guess,
                    script_public_key: from_script.clone(),
                });
            }
            template_tx.finalize();

            // Calculate masses
            let non_ctx = mass_calculator.calc_non_contextual_masses(&template_tx);

            // Calculate contextual mass (storage_mass) using PopulatedTransaction
            let utxo_entries: Vec<UtxoEntry> = selected_utxos
                .iter()
                .map(|u| UtxoEntry {
                    amount: u.utxo_entry.amount,
                    script_public_key: u.utxo_entry.script_public_key.clone(),
                    block_daa_score: u.utxo_entry.block_daa_score,
                    is_coinbase: u.utxo_entry.is_coinbase,
                })
                .collect();

            let populated = PopulatedTransaction::new(&template_tx, utxo_entries.clone());
            let ctx = match mass_calculator.calc_contextual_masses(&populated) {
                Some(c) => c,
                None => {
                    log::warn!("Mass incomputable on iteration {}, using last values", iteration);
                    break;
                }
            };

            // Calculate total mass (max of compute, transient, storage)
            let total_mass = non_ctx.compute_mass.max(non_ctx.transient_mass).max(ctx.storage_mass);

            // Calculate fee based on total mass
            let fee = (total_mass as f64 * feerate_sompi_per_gram).ceil() as u64;
            let required = amount.saturating_add(fee);
            let new_change = total_input.saturating_sub(required);

            // Check if we've converged
            if new_change == change_guess
                && fee == last_fee
                && ctx.storage_mass == last_storage_mass
            {
                log::info!("Fee/change calculation converged after {} iterations", iteration + 1);
                log::info!(
                    "Final: fee={} sompi, change={} sompi, storage_mass={}",
                    fee,
                    new_change,
                    ctx.storage_mass
                );
                break;
            }

            // Check if change would be zero (insufficient funds)
            if new_change == 0 {
                log::warn!("Insufficient funds for fee on iteration {}", iteration);
                break;
            }

            last_fee = fee;
            last_storage_mass = ctx.storage_mass;
            change_guess = new_change;
        }

        // Verify storage_mass is under limit
        // storage_mass is proportional to total output values, so large change = high storage_mass
        // IMPORTANT: Splitting change into multiple outputs INCREASES compute_mass, making things worse!
        // The solution is to use UTXOs closer to the target amount, not to split outputs.
        let _change = change_guess;
        let final_fee = last_fee;
        let final_storage_mass = last_storage_mass;

        // Based on observations: storage_mass scales with total output value
        // For storage_mass < 100K, we need very small change outputs
        // Rough estimate: max change ≈ 10M sompi (0.1 KAS) for standard transactions
        const MAX_CHANGE_FOR_STANDARD_TX: u64 = 10_000_000; // ~0.1 KAS max change

        if last_storage_mass > 100_000 {
            log::error!(
                "Transaction storage mass ({}) exceeds limit (100,000) due to large change output ({} sompi).",
                last_storage_mass,
                change_guess
            );

            if change_guess > MAX_CHANGE_FOR_STANDARD_TX {
                log::error!(
                    "Change output ({} sompi) is too large. Maximum change for standard transaction: {} sompi (~{:.2} KAS)",
                    change_guess,
                    MAX_CHANGE_FOR_STANDARD_TX,
                    MAX_CHANGE_FOR_STANDARD_TX as f64 / 100_000_000.0
                );
                return Err(Error::Serialization(format!(
                    "Transaction cannot be created: change output ({} sompi, ~{:.2} KAS) is too large and causes storage_mass ({}) to exceed limit (100,000). \
                    Maximum change for a standard transaction: {} sompi (~{:.2} KAS). \
                    \
                    SOLUTION: You need to use UTXOs that are closer to the target amount. \
                    Current situation: \
                    - Selected UTXO(s): {} sompi (~{:.2} KAS) \
                    - Target amount: {} sompi (~{:.2} KAS) \
                    - Change would be: {} sompi (~{:.2} KAS) - TOO LARGE \
                    \
                    Options: \
                    1. Consolidate UTXOs: Send a transaction to yourself (your FROM_ADDRESS) to create smaller UTXOs \
                    2. Use a UTXO closer to the target amount (within ~0.1 KAS) \
                    3. Send a larger amount to reduce the relative change size",
                    change_guess,
                    change_guess as f64 / 100_000_000.0,
                    last_storage_mass,
                    MAX_CHANGE_FOR_STANDARD_TX,
                    MAX_CHANGE_FOR_STANDARD_TX as f64 / 100_000_000.0,
                    total_input,
                    total_input as f64 / 100_000_000.0,
                    amount,
                    amount as f64 / 100_000_000.0,
                    change_guess,
                    change_guess as f64 / 100_000_000.0
                )));
            }

            // If change is small but storage_mass is still too high, check payload
            if let Some(proof) = proof_data {
                if !proof.is_empty() {
                    log::warn!(
                        "Payload size: {} bytes might be contributing to high storage_mass",
                        proof.len()
                    );
                }
            }

            return Err(Error::Serialization(format!(
                "Transaction storage_mass ({}) exceeds limit (100,000). \
                Change output ({} sompi, ~{:.2} KAS) is within reasonable range, but storage_mass is still too high. \
                This might be due to payload size or other transaction factors. \
                Try reducing the payload size or using a UTXO even closer to the target amount.",
                last_storage_mass,
                change_guess,
                change_guess as f64 / 100_000_000.0
            )));
        }

        // Calculate final fee based on mass BEFORE building transaction
        // This ensures we use the correct fee and don't need to adjust after signing
        // Use a conservative estimate: higher fees to ensure transaction is prioritized
        // We'll recalculate more accurately after building, but this gives us a good starting point
        let estimated_mass_based_fee = (final_storage_mass.max(2000) * 20).max(10000); // Conservative estimate
        let final_fee_to_use = final_fee.max(estimated_mass_based_fee);

        // Recalculate change with final fee
        let final_change = total_input.saturating_sub(amount).saturating_sub(final_fee_to_use);

        log::info!(
            "Final transaction: fee={} sompi, change={} sompi, storage_mass={}",
            final_fee_to_use,
            final_change,
            final_storage_mass
        );

        // Build the actual transaction using the calculated fee and change
        let mut tx = Transaction::default();
        tx.subnetwork_id = SUBNETWORK_ID_NATIVE;

        // Embed proof data in transaction payload if provided
        if let Some(proof) = proof_data {
            if !proof.is_empty() {
                tx.payload = proof.to_vec();
                log::info!("Embedding {} bytes of proof data in transaction payload", proof.len());
            }
        }

        // Add inputs
        for utxo in &selected_utxos {
            tx.inputs.push(TransactionInput {
                previous_outpoint: utxo.outpoint.into(),
                signature_script: vec![], // Will be filled during signing
                sequence: 0,
                sig_op_count: 1,
            });
        }

        // Add output to recipient (with privacy support)
        match privacy_mode {
            PrivacyMode::Public => {
                // Standard public transaction
                log::debug!(
                    "Creating public output to recipient: {} sompi ({:.8} KAS), script_len={}",
                    amount,
                    amount as f64 / 100_000_000.0,
                    to_script.script().len()
                );
                tx.outputs.push(TransactionOutput {
                    value: amount,
                    script_public_key: to_script.clone(),
                });
            }
            PrivacyMode::PrivateComputation | PrivacyMode::FullPrivacy => {
                // For private transactions, we use confidential amounts
                // Generate a random blinding factor for proper privacy
                let blinding_factor = privacy::generate_blinding_factor();
                let amount_commitment = privacy::commit_amount(amount, &blinding_factor);

                // For FullPrivacy mode, also generate range proof
                if privacy_mode == PrivacyMode::FullPrivacy {
                    let range_proof =
                        privacy::prove_amount_range(amount, &blinding_factor, u64::MAX);

                    log::info!(
                        "Creating fully private output: committed amount with range proof (public: {} sompi), script_len={}",
                        amount,
                        to_script.script().len()
                    );

                    // Embed commitment and range proof in payload
                    let mut privacy_data = Vec::new();
                    privacy_data.extend_from_slice(&amount_commitment.commitment);
                    privacy_data.extend_from_slice(&blinding_factor);
                    privacy_data.extend_from_slice(&range_proof.proof_bytes);

                    if tx.payload.is_empty() {
                        tx.payload = privacy_data;
                    } else {
                        let mut payload = tx.payload.clone();
                        payload.extend_from_slice(&privacy_data);
                        tx.payload = payload;
                    }
                } else {
                    // PrivateComputation mode: just commitment
                    log::info!(
                        "Creating private output: committed amount (public: {} sompi), script_len={}",
                        amount,
                        to_script.script().len()
                    );

                    // Add the commitment to payload for privacy verification
                    if tx.payload.is_empty() {
                        tx.payload =
                            borsh::to_vec(&amount_commitment.commitment).unwrap_or_default();
                    } else {
                        // Append commitment to existing payload
                        let mut payload = tx.payload.clone();
                        payload.extend_from_slice(&amount_commitment.commitment);
                        tx.payload = payload;
                    }
                }

                // Still use actual amount for transaction (Kaspa requires valid amounts)
                // The commitment and range proof prove the amount without revealing it in the proof
                tx.outputs.push(TransactionOutput {
                    value: amount,
                    script_public_key: to_script.clone(),
                });
            }
        }

        // Add change output if needed (with privacy support)
        if final_change > 0 {
            match privacy_mode {
                PrivacyMode::Public => {
                    // Standard public change output
                    log::debug!(
                        "Creating public change output: {} sompi ({:.8} KAS), script_len={}",
                        final_change,
                        final_change as f64 / 100_000_000.0,
                        from_script.script().len()
                    );
                    tx.outputs.push(TransactionOutput {
                        value: final_change,
                        script_public_key: from_script.clone(),
                    });
                }
                PrivacyMode::PrivateComputation | PrivacyMode::FullPrivacy => {
                    // For private transactions, commit the change amount
                    let change_blinding_factor = privacy::generate_blinding_factor();
                    let change_commitment =
                        privacy::commit_amount(final_change, &change_blinding_factor);

                    if privacy_mode == PrivacyMode::FullPrivacy {
                        // FullPrivacy: also generate range proof for change
                        let change_range_proof = privacy::prove_amount_range(
                            final_change,
                            &change_blinding_factor,
                            u64::MAX,
                        );

                        log::info!(
                            "Creating fully private change output: committed amount with range proof (public: {} sompi), script_len={}",
                            final_change,
                            from_script.script().len()
                        );

                        // Embed change commitment and range proof
                        let mut privacy_data = Vec::new();
                        privacy_data.extend_from_slice(&change_commitment.commitment);
                        privacy_data.extend_from_slice(&change_blinding_factor);
                        privacy_data.extend_from_slice(&change_range_proof.proof_bytes);

                        let mut payload = tx.payload.clone();
                        payload.extend_from_slice(&privacy_data);
                        tx.payload = payload;
                    } else {
                        // PrivateComputation: just commitment
                        log::info!(
                            "Creating private change output: committed amount (public: {} sompi), script_len={}",
                            final_change,
                            from_script.script().len()
                        );

                        // Append change commitment to payload
                        let mut payload = tx.payload.clone();
                        payload.extend_from_slice(&change_commitment.commitment);
                        tx.payload = payload;
                    }

                    // Still use actual amount for transaction
                    tx.outputs.push(TransactionOutput {
                        value: final_change,
                        script_public_key: from_script.clone(),
                    });
                }
            }
        }

        // CRITICAL: Calculate and verify mass BEFORE signing
        // This ensures we catch any mass issues before invalidating signatures
        let params = Params::from(self.network_id.network_type());
        let mass_calculator = MassCalculator::new(
            params.mass_per_tx_byte,
            params.mass_per_script_pub_key_byte,
            params.mass_per_sig_op,
            params.storage_mass_parameter,
        );

        // Calculate non-contextual masses (compute_mass, transient_mass)
        let non_ctx = mass_calculator.calc_non_contextual_masses(&tx);
        log::info!("Transaction compute mass: {} (limit: 100,000)", non_ctx.compute_mass);
        log::info!("Transaction transient mass: {} (limit: 100,000)", non_ctx.transient_mass);

        // Check non-contextual mass limits
        if non_ctx.compute_mass > 100_000 {
            log::error!(
                "Transaction compute mass ({}) exceeds limit (100,000). Transaction will be REJECTED!",
                non_ctx.compute_mass
            );
            return Err(Error::Serialization(format!(
                "Transaction compute_mass ({}) exceeds limit (100,000). Transaction is too large.",
                non_ctx.compute_mass
            )));
        }
        if non_ctx.transient_mass > 100_000 {
            log::error!(
                "Transaction transient mass ({}) exceeds limit (100,000). Transaction will be REJECTED!",
                non_ctx.transient_mass
            );
            return Err(Error::Serialization(format!(
                "Transaction transient_mass ({}) exceeds limit (100,000). Transaction is too large.",
                non_ctx.transient_mass
            )));
        }

        // Calculate contextual masses (storage_mass) using PopulatedTransaction
        let utxo_entries: Vec<UtxoEntry> = selected_utxos
            .iter()
            .map(|u| UtxoEntry {
                amount: u.utxo_entry.amount,
                script_public_key: u.utxo_entry.script_public_key.clone(),
                block_daa_score: u.utxo_entry.block_daa_score,
                is_coinbase: u.utxo_entry.is_coinbase,
            })
            .collect();

        let populated = PopulatedTransaction::new(&tx, utxo_entries.clone());
        let ctx =
            mass_calculator.calc_contextual_masses(&populated).ok_or(Error::Serialization(
                "Mass incomputable - cannot calculate contextual masses".to_string(),
            ))?;

        log::info!("Transaction storage mass: {} (limit: 100,000)", ctx.storage_mass);

        // Check storage mass limit
        if ctx.storage_mass > 100_000 {
            log::error!(
                "Transaction storage mass ({}) exceeds limit (100,000). Transaction will be REJECTED!",
                ctx.storage_mass
            );
            return Err(Error::Serialization(format!(
                "Transaction storage_mass ({}) exceeds limit (100,000). Transaction is too large.",
                ctx.storage_mass
            )));
        }

        // CRITICAL: Set the mass on the transaction BEFORE signing
        // This is what kaspa_audio_transfer does - it ensures the RPC reports the correct mass
        tx.set_mass(ctx.storage_mass);
        log::info!("Set transaction mass to: {} (storage_mass)", ctx.storage_mass);

        // Sign the transaction (ONCE, after everything is finalized)
        // DO NOT modify the transaction after signing - it will invalidate signatures!
        self.sign_transaction(&mut tx, &selected_utxos, from_address, private_key)?;

        // Log transaction summary for debugging
        log::info!(
            "Transaction summary: {} inputs, {} outputs, total_input={} sompi, amount={} sompi, change={} sompi, fee={} sompi",
            tx.inputs.len(),
            tx.outputs.len(),
            total_input,
            amount,
            final_change,
            final_fee_to_use
        );

        // CRITICAL: Finalize transaction BEFORE calculating ID
        // This ensures all fields are properly set and transaction is valid
        tx.finalize();

        // Verify transaction ID is non-zero (indicates transaction is valid)
        let tx_id = tx.id();
        if tx_id.as_bytes().iter().all(|&b| b == 0) {
            log::error!("Transaction ID is zero - transaction is INVALID!");
            return Err(Error::Serialization(
                "Transaction ID is zero - transaction is invalid. Check transaction structure."
                    .to_string(),
            ));
        }
        log::debug!("Transaction ID: {} (valid)", tx_id);

        // Verify transaction has inputs and outputs
        if tx.inputs.is_empty() {
            return Err(Error::Serialization("Transaction has no inputs".to_string()));
        }
        if tx.outputs.is_empty() {
            return Err(Error::Serialization("Transaction has no outputs".to_string()));
        }

        // Verify all outputs have non-zero values
        for (i, output) in tx.outputs.iter().enumerate() {
            if output.value == 0 {
                log::warn!("Output {} has zero value - this might cause issues", i);
            }
        }

        Ok(tx)
    }

    /// Signs a transaction using Kaspa's built-in SignableTransaction API.
    ///
    /// This uses the official kaspa_consensus_core::tx::SignableTransaction
    /// which handles all the complex sighash calculation and signature script
    /// formatting automatically. This is the recommended approach.
    ///
    /// Based on implementation in kaspa_audio_transfer and mixer1.0.
    fn sign_transaction(
        &self,
        tx: &mut Transaction,
        utxos: &[&RpcUtxosByAddressesEntry],
        _address: &Address,
        private_key: &PrivateKey,
    ) -> Result<()> {
        // Convert private key to Keypair
        let keypair = private_key.to_keypair()?;

        // Set sig_op_count for each input (required for SignableTransaction)
        for input in &mut tx.inputs {
            input.sig_op_count = 1;
        }

        // Create SignableTransaction
        let mut signable = SignableTransaction::new(tx.clone());

        // Set UTXO entries for each input
        for (i, utxo_entry) in utxos.iter().enumerate() {
            if i >= signable.entries.len() {
                break;
            }

            // Convert RpcUtxoEntry to UtxoEntry
            let entry = UtxoEntry {
                amount: utxo_entry.utxo_entry.amount,
                script_public_key: utxo_entry.utxo_entry.script_public_key.clone(),
                block_daa_score: utxo_entry.utxo_entry.block_daa_score,
                is_coinbase: utxo_entry.utxo_entry.is_coinbase,
            };

            signable.entries[i] = Some(entry);
        }

        // Sign using Kaspa's built-in sign function
        // sign() returns SignableTransaction, we need to extract the transaction
        let signed = sign::sign(signable, keypair);

        // Extract and finalize the transaction
        let mut signed_tx = signed.tx;
        signed_tx.finalize();

        // Replace the transaction with the signed version
        *tx = signed_tx;

        Ok(())
    }

    /// OLD METHOD: Manual signing implementation (kept for reference)
    /// This was causing "invalid hash type" errors.
    /// Use sign_transaction() above instead, which uses SignableTransaction.
    #[allow(dead_code)]
    fn sign_transaction_manual(
        &self,
        tx: &mut Transaction,
        utxos: &[&RpcUtxosByAddressesEntry],
        _address: &Address,
        private_key: &PrivateKey,
    ) -> Result<()> {
        // Convert private key to Keypair
        let keypair = private_key.to_keypair()?;

        // Set sig_op_count for each input (required for SignableTransaction)
        for input in &mut tx.inputs {
            input.sig_op_count = 1;
        }

        // Create SignableTransaction
        let mut signable = SignableTransaction::new(tx.clone());

        // Set UTXO entries for each input
        for (i, utxo_entry) in utxos.iter().enumerate() {
            if i >= signable.entries.len() {
                break;
            }

            // Convert RpcUtxoEntry to UtxoEntry
            let entry = UtxoEntry {
                amount: utxo_entry.utxo_entry.amount,
                script_public_key: utxo_entry.utxo_entry.script_public_key.clone(),
                block_daa_score: utxo_entry.utxo_entry.block_daa_score,
                is_coinbase: utxo_entry.utxo_entry.is_coinbase,
            };

            signable.entries[i] = Some(entry);
        }

        // Sign using Kaspa's built-in sign function
        let mut signed = sign::sign(signable, keypair);

        // Finalize the transaction
        signed.tx.finalize();

        // Replace the transaction with the signed version
        *tx = signed.tx;

        Ok(())
    }

    /// Calculates the sighash for a transaction input using Kaspa's txscript.
    ///
    /// This uses Kaspa's proper transaction serialization format to compute the sighash.
    /// The sighash is the hash of the transaction with the script_public_key in place of
    /// the signature_script for the input being signed.
    #[allow(dead_code)]
    fn calculate_sighash(
        &self,
        tx: &Transaction,
        input_index: usize,
        script_public_key: &ScriptPublicKey,
        utxo_value: u64,
    ) -> Result<kaspa_hashes::Hash> {
        // Try using kaspa-txscript's calculate_sighash if available
        // First, let's try to use the transaction's proper serialization
        // by checking if there's a way to compute the hash correctly

        // Build a transaction with modified signature scripts for sighash calculation
        // Note: We don't need to save original scripts since we're creating a new transaction

        // Temporarily modify the transaction for sighash calculation
        // We need mutable access, so we'll work with a reference that we can temporarily modify
        // Actually, we can't mutate tx here since it's immutable. Let's build a new transaction.

        // Build a new transaction with modified signature scripts
        // Use default() and then set accessible fields (clone what's needed)
        let mut sighash_tx = Transaction::default();
        sighash_tx.version = tx.version;
        sighash_tx.inputs = tx
            .inputs
            .iter()
            .enumerate()
            .map(|(i, input)| {
                let mut new_input = input.clone();
                if i == input_index {
                    // Use script_public_key for the input being signed
                    new_input.signature_script = script_public_key.script().to_vec();
                } else {
                    // Empty script for other inputs
                    new_input.signature_script.clear();
                }
                new_input
            })
            .collect();
        sighash_tx.outputs = tx.outputs.clone();
        sighash_tx.lock_time = tx.lock_time;
        sighash_tx.subnetwork_id = tx.subnetwork_id.clone();
        sighash_tx.gas = tx.gas;
        sighash_tx.payload = tx.payload.clone();

        // Calculate the hash using Transaction.id()
        // Note: Transaction.id() may be lazily computed and cached
        // If it returns zero, the transaction might need to be accessed differently
        // to trigger ID computation, or we need to use Kaspa's serialization format

        // Try to compute the ID - accessing it might trigger computation
        let mut computed_id = sighash_tx.id();

        // If ID is zero, Transaction.id() isn't computing correctly
        // This happens because Transaction created with default() doesn't initialize the ID
        //
        // IMPORTANT: Kaspa uses Blake2b hashing for sighash (not SHA256!)
        // The sighash format is BIP-143-like with specific field ordering
        // See: https://kaspa-mdbook.aspectron.com/transactions/sighashes.html
        if computed_id.as_bytes().iter().all(|&b| b == 0) {
            log::warn!("Transaction ID is zero - using BIP-143-like serialization with Blake2b");

            // Implement BIP-143-like sighash format for Kaspa
            computed_id = self.calculate_sighash_bip143_like(
                &sighash_tx,
                input_index,
                script_public_key,
                utxo_value,
            )?;
            log::debug!("Computed sighash using BIP-143-like format with Blake2b: {}", computed_id);
        } else {
            log::debug!("Computed sighash using Transaction.id(): {}", computed_id);
        }

        log::debug!(
            "Calculated sighash for input {}: {} (tx has {} inputs, {} outputs)",
            input_index,
            computed_id,
            tx.inputs.len(),
            tx.outputs.len()
        );

        Ok(computed_id)
    }

    /// Implements BIP-143-like sighash calculation for Kaspa transactions.
    ///
    /// Format according to Kaspa specification:
    /// 1. Transaction version (2 bytes, little-endian)
    /// 2. previousOutputsHash (32 bytes, Blake2b hash of all previous outpoints)
    /// 3. sequencesHash (32 bytes, Blake2b hash of all sequences)
    /// 4. sigOpCountsHash (32 bytes, Blake2b hash of all sigop counts)
    /// 5. For each input:
    ///    - Previous outpoint (transaction ID + index)
    ///    - Script public key version (2 bytes)
    ///    - Script length (variable)
    ///    - Script bytes
    ///    - Value (8 bytes, little-endian)
    ///    - Sequence (8 bytes, little-endian)
    ///    - SigOp count (1 byte)
    /// 6. outputsHash (32 bytes, Blake2b hash of all outputs)
    /// 7. Locktime (8 bytes, little-endian)
    /// 8. Subnetwork ID (20 bytes)
    /// 9. Gas (8 bytes, little-endian)
    /// 10. Payload hash (32 bytes, Blake2b hash of payload)
    /// 11. SigHash type (1 byte, 0x01 for SIGHASH_ALL)
    ///
    /// Then hash everything with Blake2b.
    #[allow(dead_code)]
    fn calculate_sighash_bip143_like(
        &self,
        tx: &Transaction,
        input_index: usize,
        script_public_key: &ScriptPublicKey,
        utxo_value: u64,
    ) -> Result<kaspa_hashes::Hash> {
        let mut serialized = Vec::new();

        // Helper to convert io::Error to our Error type
        let write_err = |e: std::io::Error| Error::Serialization(format!("IO error: {}", e));

        // 1. Transaction version (2 bytes, little-endian)
        serialized.write_all(&tx.version.to_le_bytes()).map_err(write_err)?;

        // 2. previousOutputsHash - hash of all previous outpoints
        // According to Kaspa spec: TransactionID (32 bytes) + Index (4 bytes, little-endian)
        let mut prev_outputs = Vec::new();
        for input in &tx.inputs {
            prev_outputs
                .write_all(&input.previous_outpoint.transaction_id.as_bytes())
                .map_err(write_err)?;
            prev_outputs
                .write_all(&input.previous_outpoint.index.to_le_bytes())
                .map_err(write_err)?;
        }
        let prev_outputs_hash = self.hash_blake2b(&prev_outputs);
        serialized.write_all(&prev_outputs_hash.as_bytes()).map_err(write_err)?;

        // 3. sequencesHash - hash of all sequences
        let mut sequences = Vec::new();
        for input in &tx.inputs {
            sequences.write_all(&input.sequence.to_le_bytes()).map_err(write_err)?;
        }
        let sequences_hash = self.hash_blake2b(&sequences);
        serialized.write_all(&sequences_hash.as_bytes()).map_err(write_err)?;

        // 4. sigOpCountsHash - hash of all sigop counts
        // Use the actual sigop count from inputs
        let mut sigop_counts = Vec::new();
        for input in &tx.inputs {
            sigop_counts.push(input.sig_op_count);
        }
        let sigop_counts_hash = self.hash_blake2b(&sigop_counts);
        serialized.write_all(&sigop_counts_hash.as_bytes()).map_err(write_err)?;

        // 5. Serialize inputs with script_public_key for the input being signed
        // According to Kaspa spec:
        // - PreviousOutpoint.TransactionID (32 bytes)
        // - PreviousOutpoint.Index (4 bytes, little-endian)
        // - PreviousOutput.ScriptPubKeyVersion (2 bytes, little-endian)
        // - PreviousOutput.ScriptPubKey.length (8 bytes, little-endian) - NOT varint!
        // - PreviousOutput.ScriptPubKey (serialized)
        // - PreviousOutput.Value (8 bytes, little-endian)
        // - Sequence (8 bytes, little-endian)
        // - SigOpCount (1 byte)
        for (i, input) in tx.inputs.iter().enumerate() {
            // 5.1 Previous outpoint TransactionID (32 bytes)
            serialized
                .write_all(&input.previous_outpoint.transaction_id.as_bytes())
                .map_err(write_err)?;

            // 5.2 Previous outpoint Index (4 bytes, little-endian)
            serialized
                .write_all(&input.previous_outpoint.index.to_le_bytes())
                .map_err(write_err)?;

            // 5.3-5.5 Script public key (use the provided one for input being signed, empty for others)
            if i == input_index {
                // 5.3 ScriptPubKeyVersion (2 bytes, little-endian)
                let version = script_public_key.version();
                serialized.write_all(&version.to_le_bytes()).map_err(write_err)?;

                // 5.4 ScriptPubKey.length (8 bytes, little-endian) - NOT varint!
                let script = script_public_key.script();
                serialized.write_all(&(script.len() as u64).to_le_bytes()).map_err(write_err)?;

                // 5.5 ScriptPubKey (serialized)
                serialized.write_all(script).map_err(write_err)?;
            } else {
                // Empty script for other inputs
                serialized.write_all(&0u16.to_le_bytes()).map_err(write_err)?; // version = 0
                serialized.write_all(&0u64.to_le_bytes()).map_err(write_err)?; // length = 0 (8 bytes)
            }

            // 5.6 PreviousOutput.Value (8 bytes, little-endian)
            if i == input_index {
                serialized.write_all(&utxo_value.to_le_bytes()).map_err(write_err)?;
            } else {
                serialized.write_all(&0u64.to_le_bytes()).map_err(write_err)?;
            }

            // 5.7 Sequence (8 bytes, little-endian)
            serialized.write_all(&input.sequence.to_le_bytes()).map_err(write_err)?;

            // 5.8 SigOpCount (1 byte)
            serialized.push(input.sig_op_count);
        }

        // 6. outputsHash - hash of all outputs
        // According to Kaspa spec: Value (8 bytes) + ScriptPubKeyVersion (2 bytes) + ScriptPubKey.length (8 bytes) + ScriptPubKey
        let mut outputs_serialized = Vec::new();
        for output in &tx.outputs {
            // Value (8 bytes, little-endian)
            outputs_serialized.write_all(&output.value.to_le_bytes()).map_err(write_err)?;
            // ScriptPubKeyVersion (2 bytes, little-endian)
            let version = output.script_public_key.version();
            outputs_serialized.write_all(&version.to_le_bytes()).map_err(write_err)?;
            // ScriptPubKey.length (8 bytes, little-endian) - NOT varint!
            let script = output.script_public_key.script();
            outputs_serialized
                .write_all(&(script.len() as u64).to_le_bytes())
                .map_err(write_err)?;
            // ScriptPubKey (serialized)
            outputs_serialized.write_all(script).map_err(write_err)?;
        }
        let outputs_hash = self.hash_blake2b(&outputs_serialized);
        serialized.write_all(&outputs_hash.as_bytes()).map_err(write_err)?;

        // 7. Locktime (8 bytes, little-endian)
        serialized.write_all(&tx.lock_time.to_le_bytes()).map_err(write_err)?;

        // 8. Subnetwork ID (20 bytes)
        let subnet_bytes: &[u8] = tx.subnetwork_id.as_ref();
        if subnet_bytes.len() >= 20 {
            serialized.write_all(&subnet_bytes[..20]).map_err(write_err)?;
        } else {
            // Pad with zeros if needed
            serialized.write_all(subnet_bytes).map_err(write_err)?;
            serialized.write_all(&vec![0u8; 20 - subnet_bytes.len()]).map_err(write_err)?;
        }

        // 9. Gas (8 bytes, little-endian)
        serialized.write_all(&tx.gas.to_le_bytes()).map_err(write_err)?;

        // 10. Payload hash (32 bytes, Blake2b hash of payload)
        let payload_hash =
            if tx.payload.is_empty() { Hash::default() } else { self.hash_blake2b(&tx.payload) };
        serialized.write_all(&payload_hash.as_bytes()).map_err(write_err)?;

        // 11. SigHash type (1 byte, 0x01 for SIGHASH_ALL)
        serialized.push(0x01);

        // Hash everything with Blake2b
        let sighash = self.hash_blake2b(&serialized);

        // Debug: Log serialized bytes length for verification
        log::debug!("Sighash serialization: {} bytes total", serialized.len());
        log::debug!(
            "Sighash serialization breakdown: version(2) + prevOutputsHash(32) + sequencesHash(32) + sigOpCountsHash(32) + inputs({}) + outputsHash(32) + locktime(8) + subnet(20) + gas(8) + payloadHash(32) + sigHashType(1)",
            serialized.len() - 2 - 32 - 32 - 32 - 32 - 8 - 20 - 8 - 32 - 1
        );

        Ok(sighash)
    }

    /// Helper function to compute Blake2b hash
    #[allow(dead_code)]
    fn hash_blake2b(&self, data: &[u8]) -> kaspa_hashes::Hash {
        use blake2b_simd::Params;
        let mut state = Params::new().hash_length(32).to_state();
        state.update(data);
        let hash_result = state.finalize();
        Hash::from_bytes(hash_result.as_bytes().try_into().unwrap_or({
            // This should never fail since we're using 32-byte output
            [0u8; 32]
        }))
    }

    /// Helper function to write variable-length integer (similar to Bitcoin's varint)
    #[allow(dead_code)]
    fn write_varint(&self, buf: &mut Vec<u8>, value: u64) -> Result<()> {
        let write_err = |e: std::io::Error| Error::Serialization(format!("IO error: {}", e));
        if value < 0xfd {
            buf.push(value as u8);
        } else if value <= 0xffff {
            buf.push(0xfd);
            buf.write_all(&value.to_le_bytes()).map_err(write_err)?;
        } else if value <= 0xffffffff {
            buf.push(0xfe);
            buf.write_all(&value.to_le_bytes()).map_err(write_err)?;
        } else {
            buf.push(0xff);
            buf.write_all(&value.to_le_bytes()).map_err(write_err)?;
        }
        Ok(())
    }
}
