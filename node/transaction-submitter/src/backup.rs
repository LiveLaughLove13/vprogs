//! Backup and recovery functionality for wallet
//!
//! Provides encrypted backup files and recovery process.

use crate::error::{Error, Result};
use crate::key_manager::KeyManager;
use base64::{engine::general_purpose, Engine as _};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

/// Backup file structure
#[derive(Serialize, Deserialize)]
pub struct WalletBackup {
    /// Backup version
    pub version: u32,
    /// Encrypted keys (base64 encoded)
    pub encrypted_keys: String,
    /// Salt for encryption (base64 encoded)
    pub salt: String,
    /// Backup timestamp
    pub timestamp: u64,
    /// Metadata
    pub metadata: BackupMetadata,
}

#[derive(Serialize, Deserialize)]
pub struct BackupMetadata {
    /// Network (mainnet, testnet, etc.)
    pub network: String,
    /// Wallet identifier
    pub wallet_id: String,
}

/// Backup manager for wallet
pub struct BackupManager {
    backup_dir: PathBuf,
}

impl BackupManager {
    /// Create a new backup manager
    pub fn new(backup_dir: impl AsRef<Path>) -> Self {
        Self { backup_dir: backup_dir.as_ref().to_path_buf() }
    }

    /// Initialize backup directory
    pub fn init(&self) -> Result<()> {
        fs::create_dir_all(&self.backup_dir).map_err(|e| {
            Error::Serialization(format!("Failed to create backup directory: {}", e))
        })?;
        Ok(())
    }

    /// Create a backup of the wallet
    pub fn create_backup(
        &self,
        key_manager: &KeyManager,
        password: &str,
        network: &str,
    ) -> Result<PathBuf> {
        // Load keys to verify password
        let keys = key_manager.load_stealth_keys(password)?;

        // Generate backup filename with timestamp
        let timestamp =
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();

        let wallet_id = hex::encode(&keys.view_public.serialize()[..8]); // First 8 bytes as ID
        let filename = format!("vprogs-wallet-backup-{}-{}.json", wallet_id, timestamp);
        let backup_path = self.backup_dir.join(&filename);

        // Read encrypted keys from key manager
        let encrypted_data = fs::read(key_manager.get_storage_path())
            .map_err(|e| Error::Serialization(format!("Failed to read keys: {}", e)))?;

        // Create backup structure
        let backup = WalletBackup {
            version: 1,
            encrypted_keys: general_purpose::STANDARD.encode(&encrypted_data),
            salt: general_purpose::STANDARD.encode(b"vprogs-wallet-salt"), // In production, use random salt
            timestamp,
            metadata: BackupMetadata { network: network.to_string(), wallet_id },
        };

        // Serialize to JSON
        let backup_json = serde_json::to_string_pretty(&backup)
            .map_err(|e| Error::Serialization(format!("Failed to serialize backup: {}", e)))?;

        // Write backup file
        fs::write(&backup_path, backup_json)
            .map_err(|e| Error::Serialization(format!("Failed to write backup: {}", e)))?;

        Ok(backup_path)
    }

    /// Restore wallet from backup
    pub fn restore_backup(
        &self,
        backup_path: impl AsRef<Path>,
        key_manager: &KeyManager,
        password: &str,
    ) -> Result<()> {
        // Read backup file
        let backup_json = fs::read_to_string(backup_path)
            .map_err(|e| Error::Serialization(format!("Failed to read backup: {}", e)))?;

        // Deserialize backup
        let backup: WalletBackup = serde_json::from_str(&backup_json)
            .map_err(|e| Error::Serialization(format!("Failed to parse backup: {}", e)))?;

        // Decode encrypted keys
        let encrypted_data = general_purpose::STANDARD
            .decode(&backup.encrypted_keys)
            .map_err(|e| Error::Serialization(format!("Failed to decode backup: {}", e)))?;

        // Write encrypted keys to key manager location
        fs::write(key_manager.get_storage_path(), &encrypted_data)
            .map_err(|e| Error::Serialization(format!("Failed to restore keys: {}", e)))?;

        // Verify restoration by loading keys
        key_manager.load_stealth_keys(password)?;

        Ok(())
    }

    /// List available backups
    pub fn list_backups(&self) -> Result<Vec<PathBuf>> {
        let mut backups = Vec::new();

        if !self.backup_dir.exists() {
            return Ok(backups);
        }

        for entry in fs::read_dir(&self.backup_dir)
            .map_err(|e| Error::Serialization(format!("Failed to read backup directory: {}", e)))?
        {
            let entry =
                entry.map_err(|e| Error::Serialization(format!("Failed to read entry: {}", e)))?;
            let path = entry.path();

            if path.extension().and_then(|s| s.to_str()) == Some("json")
                && path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .map(|n| n.starts_with("vprogs-wallet-backup-"))
                    .unwrap_or(false)
            {
                backups.push(path);
            }
        }

        // Sort by modification time (newest first)
        backups.sort_by(|a, b| {
            let a_time =
                fs::metadata(a).and_then(|m| m.modified()).unwrap_or(std::time::UNIX_EPOCH);
            let b_time =
                fs::metadata(b).and_then(|m| m.modified()).unwrap_or(std::time::UNIX_EPOCH);
            b_time.cmp(&a_time)
        });

        Ok(backups)
    }
}
