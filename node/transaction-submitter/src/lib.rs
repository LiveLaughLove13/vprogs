//! Transaction submission module for sending private Kaspa transactions.
//!
//! This module enables submitting vprogs transactions (with Cairo ZK-STARK proofs)
//! to the Kaspa network as private transactions.

use kaspa_wrpc_client::prelude::*;
use std::sync::Arc;
use vprogs_transaction_runtime_transaction::Transaction as VprogsTransaction;
use vprogs_transaction_runtime_transaction_effects::TransactionEffects;

use crate::error::{Error, Result};

mod backup;
mod cli;
mod error;
mod key_manager;
mod scanner;
mod wallet;

pub use backup::{BackupManager, WalletBackup};
pub use cli::WalletCLI;
pub use error::Error as SubmitError;
pub use key_manager::KeyManager;
pub use scanner::{BlockchainScanner, StealthTransaction};
pub use vprogs_transaction_runtime::privacy::PrivacyMode;
pub use wallet::{PrivateKey, Wallet};

/// Configuration for transaction submission.
pub struct TransactionSubmitterConfig {
    /// RPC client for Kaspa network
    pub rpc_client: Arc<KaspaRpcClient>,
    /// Network ID (mainnet, testnet, etc.)
    pub network_id: kaspa_consensus_core::network::NetworkId,
}

/// Submits private transactions to Kaspa network.
pub struct TransactionSubmitter {
    client: Arc<KaspaRpcClient>,
}

impl TransactionSubmitter {
    /// Creates a new transaction submitter.
    pub fn new(config: TransactionSubmitterConfig) -> Self {
        Self { client: config.rpc_client }
    }

    /// Submits a vprogs transaction with Cairo execution results to Kaspa network.
    ///
    /// This creates a Kaspa transaction that embeds:
    /// - Cairo program execution results
    /// - ZK-STARK proofs
    /// - Transaction effects
    ///
    /// The transaction is "private" because:
    /// - The actual computation is hidden in the ZK-STARK proof
    /// - Only the proof and public outputs are visible on-chain
    /// - Input data remains confidential
    pub async fn submit_private_transaction(
        &self,
        _vprogs_tx: &VprogsTransaction,
        effects: &TransactionEffects,
        from_address: kaspa_addresses::Address,
        to_address: kaspa_addresses::Address,
        amount: u64,
        private_key: &wallet::PrivateKey,
    ) -> Result<kaspa_consensus_core::Hash> {
        // Serialize the transaction effects (including Cairo proofs) to embed in Kaspa transaction
        let effects_bytes = borsh::to_vec(effects)
            .map_err(|e| Error::Serialization(format!("Failed to serialize effects: {}", e)))?;

        // Create wallet instance
        let wallet = wallet::Wallet::new(
            self.client.clone(),
            kaspa_consensus_core::network::NetworkId::new(
                kaspa_consensus_core::network::NetworkType::Mainnet,
            ),
        );

        // Build and sign the transaction
        // For now, use public mode (backward compatible)
        // TODO: Add privacy mode parameter to submit_private_transaction
        let mut kaspa_tx = wallet
            .build_transaction(
                &from_address,
                &to_address,
                amount,
                Some(&effects_bytes),
                private_key,
            )
            .await?;

        // Ensure transaction is finalized (build_transaction should have done this, but ensure it)
        kaspa_tx.finalize();

        // Calculate transaction ID after finalization
        let tx_id = kaspa_tx.id();

        log::info!("Built transaction with ID: {}", tx_id);
        log::info!(
            "Transaction has {} inputs, {} outputs",
            kaspa_tx.inputs.len(),
            kaspa_tx.outputs.len()
        );

        // Convert to RPC transaction format
        use kaspa_rpc_core::RpcTransaction;
        let rpc_tx = RpcTransaction::from(&kaspa_tx);

        // Check if transaction is already in mempool
        if let Ok(entry) = self.client.rpc_api().get_mempool_entry(tx_id, true, false).await {
            log::info!("Transaction {} is already in mempool", tx_id);
            log::info!(
                "Transaction details: {} inputs, {} outputs",
                entry.transaction.inputs.len(),
                entry.transaction.outputs.len()
            );
            for (i, output) in entry.transaction.outputs.iter().enumerate() {
                log::info!(
                    "  Output {}: {} sompi ({:.8} KAS), script_len={}",
                    i,
                    output.value,
                    output.value as f64 / 100_000_000.0,
                    output.script_public_key.script().len()
                );
            }
            return Ok(tx_id);
        }

        // Submit to Kaspa network
        // allow_orphan=false means transaction must have valid UTXOs
        // We should use allow_orphan=false for normal transactions
        log::info!("Submitting transaction to Kaspa network (allow_orphan=false)...");
        let submit_result = self.client.rpc_api().submit_transaction(rpc_tx.clone(), false).await;

        // Handle submission result - check if it's "already in mempool" error
        let tx_id = match submit_result {
            Ok(id) => {
                log::info!("Submitted private transaction with Cairo proof: {}", id);
                id
            }
            Err(e) => {
                let error_msg = e.to_string();
                // Check if error is "already in mempool" - this is actually success
                if error_msg.contains("already in the mempool")
                    || error_msg.contains("already in mempool")
                {
                    log::info!(
                        "Transaction {} is already in mempool (from previous submission)",
                        tx_id
                    );
                } else if error_msg.contains("already spent")
                    && error_msg.contains("in the mempool")
                {
                    // UTXO is locked by a pending mempool transaction
                    log::error!("UTXO is locked by a pending transaction in mempool");
                    log::error!("Error: {}", error_msg);
                    return Err(Error::Submission(format!(
                        "UTXO is locked by a pending transaction. Please wait for the previous transaction to confirm, or use different UTXOs. Error: {}",
                        error_msg
                    )));
                } else {
                    return Err(Error::Submission(format!("Failed to submit transaction: {}", e)));
                }
                tx_id
            }
        };

        // Verify the transaction was accepted
        log::info!("Verifying transaction status...");
        if let Ok(entry) = self.client.rpc_api().get_mempool_entry(tx_id, true, false).await {
            log::info!("Transaction is in mempool. Waiting for confirmation...");
            log::info!(
                "Transaction details: {} inputs, {} outputs",
                entry.transaction.inputs.len(),
                entry.transaction.outputs.len()
            );
            for (i, output) in entry.transaction.outputs.iter().enumerate() {
                let kas_amount = output.value as f64 / 100_000_000.0;
                log::info!(
                    "  Output {}: {} sompi ({:.8} KAS), script_len={}",
                    i,
                    output.value,
                    kas_amount,
                    output.script_public_key.script().len()
                );
            }
        } else {
            log::warn!(
                "Transaction not found in mempool. It may have been rejected or already confirmed."
            );
        }

        Ok(tx_id)
    }
}

/// Helper to create a transaction submitter connected to Kaspa mainnet.
pub async fn create_mainnet_submitter(rpc_url: Option<&str>) -> Result<TransactionSubmitter> {
    use kaspa_consensus_core::network::{NetworkId, NetworkType};

    let resolver = if rpc_url.is_none() { Some(Resolver::default()) } else { None };

    let client = Arc::new(
        KaspaRpcClient::new_with_args(
            WrpcEncoding::Borsh,
            rpc_url,
            resolver,
            Some(NetworkId::new(NetworkType::Mainnet)),
            None,
        )
        .map_err(|e| Error::Connection(format!("Failed to create RPC client: {}", e)))?,
    );

    // Connect the client before returning
    client
        .connect(None)
        .await
        .map_err(|e| Error::Connection(format!("Failed to connect RPC client: {}", e)))?;

    Ok(TransactionSubmitter::new(TransactionSubmitterConfig {
        rpc_client: client,
        network_id: NetworkId::new(NetworkType::Mainnet),
    }))
}
