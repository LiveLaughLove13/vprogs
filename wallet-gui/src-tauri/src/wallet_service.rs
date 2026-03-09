//! Wallet service layer for GUI
//!
//! This module wraps the CLI wallet functionality and provides
//! a clean interface for the GUI frontend.

use crate::error::{Result, WalletError};
use kaspa_consensus_core::network::{NetworkId, NetworkType};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use vprogs_node_transaction_submitter::{BlockchainScanner, KeyManager, WalletCLI};

/// Wallet service state
pub struct WalletService {
    wallet_dir: PathBuf,
    rpc_url: Option<String>,
    _network_id: NetworkId,
    wallet_cli: Arc<Mutex<Option<WalletCLI>>>,
    is_unlocked: Arc<Mutex<bool>>,
    password: Arc<Mutex<Option<String>>>, // Store password for operations
}

impl WalletService {
    pub fn new(wallet_dir: PathBuf, rpc_url: Option<String>) -> Self {
        Self {
            wallet_dir,
            rpc_url,
            _network_id: NetworkId::new(NetworkType::Mainnet),
            wallet_cli: Arc::new(Mutex::new(None)),
            is_unlocked: Arc::new(Mutex::new(false)),
            password: Arc::new(Mutex::new(None)),
        }
    }

    /// Check if wallet is initialized
    pub fn is_initialized(&self) -> bool {
        let keys_path = self.wallet_dir.join("keys.encrypted");
        keys_path.exists()
    }

    /// Check if wallet is unlocked
    pub fn is_unlocked(&self) -> bool {
        *self.is_unlocked.lock().unwrap()
    }

    /// Initialize a new wallet
    pub async fn init_wallet(&self, password: String) -> Result<()> {
        if self.is_initialized() {
            return Err(WalletError::Other("Wallet already initialized".to_string()));
        }

        let wallet_cli = WalletCLI::new(self.wallet_dir.clone(), self.rpc_url.as_deref()).await
            .map_err(|e| WalletError::Other(e.to_string()))?;

        wallet_cli.init(&password)
            .map_err(|e| WalletError::Other(e.to_string()))?;

        *self.is_unlocked.lock().unwrap() = true;
        *self.password.lock().unwrap() = Some(password.clone());
        *self.wallet_cli.lock().unwrap() = Some(wallet_cli);

        Ok(())
    }

    /// Import a wallet from a Kaspa private key
    pub async fn import_wallet_from_private_key(
        &self,
        private_key_hex: String,
        password: String,
    ) -> Result<String> {
        if self.is_initialized() {
            return Err(WalletError::Other("Wallet already initialized".to_string()));
        }

        let key_manager = KeyManager::new(self.wallet_dir.join("keys.encrypted"));
        key_manager.init()
            .map_err(|e| WalletError::Other(e.to_string()))?;

        // Derive network ID (default to mainnet)
        let network_id = NetworkId::new(NetworkType::Mainnet); // TODO: Get from settings

        // Store the imported private key and derive address
        // If address derivation fails, we'll still store the key and return a placeholder
        let address = match key_manager.store_private_key(&private_key_hex, &password, &network_id) {
            Ok(addr) => addr,
            Err(e) => {
                // If address derivation failed, log it but don't fail the import
                // The user can manually provide their address later
                eprintln!("Warning: Address derivation failed: {}. Key stored successfully.", e);
                "address_derivation_failed_please_enter_manually".to_string()
            }
        };

        // Also generate and store a stealth keypair for receiving private transactions
        // This allows the wallet to receive both regular and stealth transactions
        use vprogs_transaction_runtime::privacy::StealthKeypair;
        let stealth_keys = StealthKeypair::new();
        key_manager.store_stealth_keys(&stealth_keys, &password)
            .map_err(|e| WalletError::Other(e.to_string()))?;

        // Initialize wallet CLI with timeout to prevent hanging
        // WalletCLI::new might try to connect to RPC server, which could hang if server is unavailable
        let wallet_cli_result = tokio::time::timeout(
            tokio::time::Duration::from_secs(10),
            WalletCLI::new(self.wallet_dir.clone(), self.rpc_url.as_deref())
        ).await;
        
        let wallet_cli = match wallet_cli_result {
            Ok(Ok(cli)) => cli,
            Ok(Err(e)) => {
                eprintln!("WalletCLI::new failed: {}", e);
                return Err(WalletError::Other(format!("Failed to initialize wallet CLI: {}. Make sure RPC server is running.", e)));
            },
            Err(_) => {
                eprintln!("WalletCLI::new timed out after 10 seconds");
                return Err(WalletError::Other("Wallet CLI initialization timed out. The RPC server might not be running or is unreachable. Please check your RPC connection in settings.".to_string()));
            },
        };

        *self.is_unlocked.lock().unwrap() = true;
        *self.password.lock().unwrap() = Some(password);
        *self.wallet_cli.lock().unwrap() = Some(wallet_cli);

        Ok(address)
    }

    /// Import a wallet from stealth keys
    pub async fn import_wallet_from_stealth_keys(
        &self,
        view_secret_hex: String,
        spend_secret_hex: String,
        password: String,
    ) -> Result<()> {
        if self.is_initialized() {
            return Err(WalletError::Other("Wallet already initialized".to_string()));
        }

        let key_manager = KeyManager::new(self.wallet_dir.join("keys.encrypted"));
        key_manager.init()
            .map_err(|e| WalletError::Other(e.to_string()))?;

        // Store the imported stealth keys
        key_manager.store_stealth_keys_from_import(&view_secret_hex, &spend_secret_hex, &password)
            .map_err(|e| WalletError::Other(e.to_string()))?;

        // Initialize wallet CLI
        let wallet_cli = WalletCLI::new(self.wallet_dir.clone(), self.rpc_url.as_deref()).await
            .map_err(|e| WalletError::Other(e.to_string()))?;

        *self.is_unlocked.lock().unwrap() = true;
        *self.password.lock().unwrap() = Some(password);
        *self.wallet_cli.lock().unwrap() = Some(wallet_cli);

        Ok(())
    }

    /// Unlock wallet with password
    pub async fn unlock_wallet(&self, password: String) -> Result<()> {
        if !self.is_initialized() {
            return Err(WalletError::NotInitialized);
        }

        // Try to load keys to verify password
        let key_manager = KeyManager::new(self.wallet_dir.join("keys.encrypted"));
        let _keys = key_manager
            .load_stealth_keys(&password)
            .map_err(|_| WalletError::InvalidPassword)?;

        let wallet_cli = WalletCLI::new(self.wallet_dir.clone(), self.rpc_url.as_deref()).await
            .map_err(|e| WalletError::Other(e.to_string()))?;

        *self.is_unlocked.lock().unwrap() = true;
        *self.password.lock().unwrap() = Some(password);
        *self.wallet_cli.lock().unwrap() = Some(wallet_cli);

        Ok(())
    }

    /// Lock wallet
    pub fn lock_wallet(&self) {
        *self.is_unlocked.lock().unwrap() = false;
        *self.password.lock().unwrap() = None;
        *self.wallet_cli.lock().unwrap() = None;
    }

    /// Delete/reset wallet (removes all wallet files)
    pub fn delete_wallet(&self) -> Result<()> {
        use std::fs;
        
        // Delete the keys file
        let keys_path = self.wallet_dir.join("keys.encrypted");
        if keys_path.exists() {
            fs::remove_file(&keys_path)
                .map_err(|e| WalletError::Io(e))?;
        }

        // Delete backup directory if it exists
        let backup_dir = self.wallet_dir.join("backups");
        if backup_dir.exists() {
            fs::remove_dir_all(&backup_dir)
                .map_err(|e| WalletError::Io(e))?;
        }

        // Lock the wallet
        self.lock_wallet();

        Ok(())
    }

    /// Get wallet CLI (must be unlocked)
    fn get_wallet_cli(&self) -> Result<Arc<Mutex<Option<WalletCLI>>>> {
        if !self.is_unlocked() {
            return Err(WalletError::WalletLocked);
        }
        Ok(self.wallet_cli.clone())
    }

    /// Get password (must be unlocked)
    fn get_password(&self) -> Result<String> {
        if !self.is_unlocked() {
            return Err(WalletError::WalletLocked);
        }
        let password = self.password.lock().unwrap();
        password.clone().ok_or(WalletError::WalletLocked)
    }

    /// Get meta-address for receiving funds
    pub async fn get_meta_address(&self) -> Result<MetaAddressData> {
        let wallet_cli_arc = self.get_wallet_cli()?;
        let password = self.get_password()?;
        let wallet_cli = wallet_cli_arc.lock().unwrap();
        let wallet_cli = wallet_cli.as_ref().ok_or(WalletError::WalletLocked)?;

        let keys = wallet_cli.load(&password)
            .map_err(|e| WalletError::Other(e.to_string()))?;

        Ok(MetaAddressData {
            view_public: hex::encode(keys.view_public.serialize()),
            spend_public: hex::encode(keys.spend_public.serialize()),
            meta_address_json: serde_json::json!({
                "view_public": hex::encode(keys.view_public.serialize()),
                "spend_public": hex::encode(keys.spend_public.serialize()),
            }).to_string(),
        })
    }

    /// Get balance (includes both imported address and stealth addresses)
    pub async fn get_balance(&self) -> Result<u64> {
        let (client, keys, imported_address) = {
            let wallet_cli_arc = self.get_wallet_cli()?;
            let password = self.get_password()?;
            let wallet_cli_guard = wallet_cli_arc.lock().unwrap();
            let wallet_cli = wallet_cli_guard.as_ref().ok_or(WalletError::WalletLocked)?;

            let keys = wallet_cli
                .load(&password)
                .map_err(|e| WalletError::Other(e.to_string()))?;

            // Get imported address if available
            let key_manager = KeyManager::new(self.wallet_dir.join("keys.encrypted"));
            let imported_address = key_manager.load_address(&password)
                .map_err(|e| WalletError::Other(e.to_string()))?;

            // Return data needed for scanning; guard is dropped at end of this block
            (wallet_cli.get_client(), keys, imported_address)
        };

        let mut total_balance = 0u64;

        // Calculate balance from stealth addresses
        let scanner = BlockchainScanner::new(client.clone(), keys);
        let stealth_balance = scanner.calculate_balance().await
            .map_err(|e| WalletError::Other(e.to_string()))?;
        total_balance += stealth_balance;

        // Calculate balance from imported address if available
        if let Some(address_str) = imported_address {
            use kaspa_addresses::Address;
            if let Ok(address) = Address::try_from(address_str.as_str()) {
                let utxos = client.rpc_api()
                    .get_utxos_by_addresses(vec![address])
                    .await
                    .map_err(|e| WalletError::Other(format!("Failed to get UTXOs: {}", e)))?;
                
                let address_balance: u64 = utxos.iter()
                    .map(|entry| entry.utxo_entry.amount)
                    .sum();
                total_balance += address_balance;
            }
        }

        Ok(total_balance)
    }

    /// Get the imported Kaspa address (if any)
    pub async fn get_imported_address(&self) -> Result<Option<String>> {
        let password = self.get_password()?;
        let key_manager = KeyManager::new(self.wallet_dir.join("keys.encrypted"));
        key_manager.load_address(&password)
            .map_err(|e| WalletError::Other(e.to_string()))
    }

    /// Get transactions
    pub async fn get_transactions(&self) -> Result<Vec<TransactionInfo>> {
        let (client, keys) = {
            let wallet_cli_arc = self.get_wallet_cli()?;
            let password = self.get_password()?;
            let wallet_cli_guard = wallet_cli_arc.lock().unwrap();
            let wallet_cli = wallet_cli_guard.as_ref().ok_or(WalletError::WalletLocked)?;

            let keys = wallet_cli
                .load(&password)
                .map_err(|e| WalletError::Other(e.to_string()))?;

            (wallet_cli.get_client(), keys)
        };

        let scanner = BlockchainScanner::new(client, keys);
        let stealth_txs = scanner.scan_mempool().await
            .map_err(|e| WalletError::Other(e.to_string()))?;

        Ok(stealth_txs.into_iter().map(|tx| TransactionInfo {
            id: tx.tx_id,
            amount: tx.amount,
            timestamp: 0, // TODO: Get actual timestamp
            privacy_mode: "full_privacy".to_string(),
            status: if tx.block_daa_score.is_some() { "confirmed".to_string() } else { "pending".to_string() },
        }).collect())
    }

    /// Send transaction
    pub async fn send_transaction(
        &self,
        _to_address: String,
        _amount: u64,
        _privacy_mode: String,
        _from_address: String,
    ) -> Result<String> {
        // TODO: Implement full transaction building and submission with privacy modes
        // For now, this is a stub that returns a placeholder transaction ID
        Ok("transaction_id_placeholder".to_string())
    }
}

/// Meta-address data for GUI
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct MetaAddressData {
    pub view_public: String,
    pub spend_public: String,
    pub meta_address_json: String,
}

/// Transaction information for GUI
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct TransactionInfo {
    pub id: String,
    pub amount: u64,
    pub timestamp: u64,
    pub privacy_mode: String,
    pub status: String,
}

