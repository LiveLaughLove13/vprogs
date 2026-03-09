// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod wallet_service;
mod error;
mod settings;

use commands::*;

use wallet_service::WalletService;
use settings::SettingsManager;
use std::path::PathBuf;
use tokio::sync::Mutex;
use dirs::home_dir;

fn main() {
    // Determine wallet directory (default: ~/.vprogs-wallet)
    let wallet_dir = home_dir()
        .map(|h| h.join(".vprogs-wallet"))
        .unwrap_or_else(|| PathBuf::from(".vprogs-wallet"));

    // Create settings manager
    let config_path = wallet_dir.join("config.json");
    let settings_manager = SettingsManager::new(config_path);
    
    // Load settings
    let settings = settings_manager.load().unwrap_or_else(|e| {
        eprintln!("Warning: Failed to load settings: {}. Using defaults.", e);
        settings::Settings::default()
    });

    // Create wallet service with settings
    let wallet_service = WalletService::new(wallet_dir.clone(), settings.rpc_url.clone());

    tauri::Builder::default()
        .manage(Mutex::new(wallet_service))
        .manage(Mutex::new(settings_manager))
        .invoke_handler(tauri::generate_handler![
            // Wallet management
            init_wallet,
            import_wallet_from_private_key,
            import_wallet_from_stealth_keys,
            unlock_wallet,
            lock_wallet,
            delete_wallet,
            is_wallet_initialized,
            is_wallet_unlocked,
            // Address management
            get_receiving_address,
            get_meta_address,
            get_imported_address,
            generate_meta_address,
            // Balance & transactions
            get_balance,
            get_transactions,
            scan_blockchain,
            // Send transactions
            send_transaction,
            estimate_fee,
            // Settings
            get_settings,
            update_settings,
            // Backup & restore
            create_backup,
            restore_backup,
            list_backups,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

