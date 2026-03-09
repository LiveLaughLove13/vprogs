//! Command-line interface for the wallet
//!
//! Provides user-friendly commands for wallet operations.

use crate::backup::{BackupManager, WalletBackup};
use crate::error::{Error, Result};
use crate::key_manager::KeyManager;
use crate::scanner::BlockchainScanner;
use kaspa_addresses::Address;
use kaspa_consensus_core::network::{NetworkId, NetworkType};
use kaspa_wrpc_client::prelude::*;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use vprogs_transaction_runtime::privacy::StealthKeypair;

/// Main wallet CLI structure
pub struct WalletCLI {
    key_manager: KeyManager,
    backup_manager: BackupManager,
    client: Arc<KaspaRpcClient>,
    _network_id: NetworkId,
    _wallet_dir: PathBuf,
}

impl WalletCLI {
    /// Get a reference to the RPC client
    pub fn get_client(&self) -> Arc<KaspaRpcClient> {
        self.client.clone()
    }

    /// Initialize a new wallet CLI
    pub async fn new(wallet_dir: PathBuf, rpc_url: Option<&str>) -> Result<Self> {
        // Setup key manager
        let keys_path = wallet_dir.join("keys.encrypted");
        let key_manager = KeyManager::new(keys_path);
        key_manager.init()?;

        // Setup RPC client
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

        client
            .connect(None)
            .await
            .map_err(|e| Error::Connection(format!("Failed to connect: {}", e)))?;

        // Setup backup manager
        let backup_dir = wallet_dir.join("backups");
        let backup_manager = BackupManager::new(backup_dir);
        backup_manager.init()?;

        Ok(Self {
            key_manager,
            backup_manager,
            client,
            _network_id: NetworkId::new(NetworkType::Mainnet),
            _wallet_dir: wallet_dir,
        })
    }

    /// Initialize a new wallet (generate keys)
    pub fn init(&self, password: &str) -> Result<()> {
        if self.key_manager.keys_exist() {
            return Err(Error::Serialization(
                "Wallet already exists. Use 'load' to unlock.".to_string(),
            ));
        }

        println!("Generating new stealth keypair...");
        let keys = StealthKeypair::new();

        println!("Encrypting and storing keys...");
        self.key_manager.store_stealth_keys(&keys, password)?;

        println!("✅ Wallet initialized successfully!");
        println!();
        println!("Your meta-address (share this to receive funds):");
        println!("  View Public:  {}", hex::encode(keys.view_public.serialize()));
        println!("  Spend Public: {}", hex::encode(keys.spend_public.serialize()));

        Ok(())
    }

    /// Load wallet keys (unlock with password)
    pub fn load(&self, password: &str) -> Result<StealthKeypair> {
        if !self.key_manager.keys_exist() {
            return Err(Error::Serialization(
                "Wallet not initialized. Use 'init' first.".to_string(),
            ));
        }

        self.key_manager
            .load_stealth_keys(password)
            .map_err(|e| Error::Serialization(format!("Failed to load keys: {}", e)))
    }

    /// Show meta-address for receiving funds
    pub fn receive(&self, password: &str) -> Result<()> {
        let keys = self.load(password)?;

        println!("Your Meta-Address (share this to receive funds):");
        println!("  View Public:  {}", hex::encode(keys.view_public.serialize()));
        println!("  Spend Public: {}", hex::encode(keys.spend_public.serialize()));
        println!();
        println!("Format (JSON):");
        println!("{{");
        println!("  \"view_public\": \"{}\",", hex::encode(keys.view_public.serialize()));
        println!("  \"spend_public\": \"{}\"", hex::encode(keys.spend_public.serialize()));
        println!("}}");

        Ok(())
    }

    /// Show balance
    pub async fn balance(&self, password: &str) -> Result<()> {
        let keys = self.load(password)?;
        let scanner = BlockchainScanner::new(self.client.clone(), keys);

        println!("Scanning blockchain for your transactions...");
        let balance = scanner.calculate_balance().await?;

        println!("Balance: {} sompi ({:.8} KAS)", balance, balance as f64 / 100_000_000.0);

        Ok(())
    }

    /// List transactions
    pub async fn transactions(&self, password: &str) -> Result<()> {
        let keys = self.load(password)?;
        let scanner = BlockchainScanner::new(self.client.clone(), keys);

        println!("Scanning blockchain for your transactions...");
        let txs = scanner.scan_mempool().await?;

        if txs.is_empty() {
            println!("No transactions found.");
            return Ok(());
        }

        println!("Found {} transaction(s):", txs.len());
        println!();
        for (i, tx) in txs.iter().enumerate() {
            println!("Transaction {}:", i + 1);
            println!("  ID: {}", tx.tx_id);
            println!("  Amount: {} sompi ({:.8} KAS)", tx.amount, tx.amount as f64 / 100_000_000.0);
            println!("  Output Index: {}", tx.output_index);
            if let Some(daa) = tx.block_daa_score {
                println!("  Block DAA Score: {}", daa);
            } else {
                println!("  Status: Pending (mempool)");
            }
            println!();
        }

        Ok(())
    }

    /// Send funds
    pub async fn send(
        &self,
        password: &str,
        to_address: &str,
        amount: u64,
        from_address: &str,
    ) -> Result<()> {
        let _keys = self.load(password)?;

        // Parse addresses
        let _to_addr = Address::try_from(to_address)
            .map_err(|e| Error::Serialization(format!("Invalid destination address: {}", e)))?;
        let _from_addr = Address::try_from(from_address)
            .map_err(|e| Error::Serialization(format!("Invalid source address: {}", e)))?;

        // Get private key for from_address
        // For now, we'll need the private key - in production, derive from stealth keys
        // This is a simplified version - full implementation would handle stealth address spending
        println!("⚠️  Note: Full stealth address spending requires transaction details");
        println!("   Use the spend_from_stealth_address example for now");

        // For this simplified version, we'll just show what would happen
        println!(
            "Would send {} sompi ({:.8} KAS) to {}",
            amount,
            amount as f64 / 100_000_000.0,
            to_address
        );
        println!("From: {}", from_address);

        Ok(())
    }

    /// Create a backup of the wallet
    pub fn backup(&self, password: &str) -> Result<()> {
        let _keys = self.load(password)?;

        println!("Creating backup...");
        let backup_path =
            self.backup_manager.create_backup(&self.key_manager, password, "mainnet")?;

        println!("✅ Backup created successfully!");
        println!("   Location: {}", backup_path.display());

        Ok(())
    }

    /// Restore wallet from backup
    pub fn restore(&self, backup_path: impl AsRef<std::path::Path>, password: &str) -> Result<()> {
        println!("Restoring wallet from backup...");

        self.backup_manager.restore_backup(backup_path, &self.key_manager, password)?;

        println!("✅ Wallet restored successfully!");
        println!(
            "   You can now use your wallet with: vprogs-wallet receive --password <password>"
        );

        Ok(())
    }

    /// List available backups
    pub fn list_backups(&self) -> Result<()> {
        let backups = self.backup_manager.list_backups()?;

        if backups.is_empty() {
            println!("No backups found.");
            return Ok(());
        }

        println!("Available backups ({}):", backups.len());
        println!();

        for (i, backup_path) in backups.iter().enumerate() {
            if let Ok(backup_json) = fs::read_to_string(backup_path) {
                if let Ok(backup) = serde_json::from_str::<WalletBackup>(&backup_json) {
                    // Format timestamp as date string
                    let date =
                        chrono::DateTime::<chrono::Utc>::from_timestamp(backup.timestamp as i64, 0)
                            .map(|dt| dt.format("%Y-%m-%d %H:%M:%S UTC").to_string())
                            .unwrap_or_else(|| "Unknown".to_string());

                    println!("  {}. {}", i + 1, backup_path.file_name().unwrap().to_string_lossy());
                    println!("     Wallet ID: {}", backup.metadata.wallet_id);
                    println!("     Network: {}", backup.metadata.network);
                    println!("     Date: {}", date);
                    println!();
                }
            }
        }

        Ok(())
    }
}
