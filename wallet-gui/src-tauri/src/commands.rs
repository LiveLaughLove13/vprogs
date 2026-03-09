//! Tauri commands (IPC handlers) for wallet operations

use crate::wallet_service::{TransactionInfo, WalletService};
use crate::settings::{Settings, SettingsManager};
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;
use tauri::State;

// Global wallet service state
type WalletState<'a> = State<'a, Mutex<WalletService>>;

// ============================================================================
// Wallet Management Commands
// ============================================================================

#[derive(Debug, Serialize, Deserialize)]
pub struct InitWalletRequest {
    password: String,
}

#[tauri::command]
pub async fn init_wallet(
    request: InitWalletRequest,
    wallet: WalletState<'_>,
) -> Result<(), String> {
    let wallet_service = wallet.lock().await;
    wallet_service
        .init_wallet(request.password)
        .await
        .map_err(|e| e.to_string())
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ImportPrivateKeyRequest {
    private_key: String, // Hex string (64 characters)
    password: String,
}

#[tauri::command]
pub async fn import_wallet_from_private_key(
    request: ImportPrivateKeyRequest,
    wallet: WalletState<'_>,
) -> Result<String, String> {
    let wallet_service = wallet.lock().await;
    wallet_service
        .import_wallet_from_private_key(request.private_key, request.password)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_imported_address(wallet: WalletState<'_>) -> Result<Option<String>, String> {
    let wallet_service = wallet.lock().await;
    wallet_service
        .get_imported_address()
        .await
        .map_err(|e| e.to_string())
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ImportStealthKeysRequest {
    view_secret: String,  // Hex string (64 characters)
    spend_secret: String, // Hex string (64 characters)
    password: String,
}

#[tauri::command]
pub async fn import_wallet_from_stealth_keys(
    request: ImportStealthKeysRequest,
    wallet: WalletState<'_>,
) -> Result<(), String> {
    let wallet_service = wallet.lock().await;
    wallet_service
        .import_wallet_from_stealth_keys(
            request.view_secret,
            request.spend_secret,
            request.password,
        )
        .await
        .map_err(|e| e.to_string())
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UnlockWalletRequest {
    password: String,
}

#[tauri::command]
pub async fn unlock_wallet(
    request: UnlockWalletRequest,
    wallet: WalletState<'_>,
) -> Result<(), String> {
    let wallet_service = wallet.lock().await;
    wallet_service
        .unlock_wallet(request.password)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn lock_wallet(wallet: WalletState<'_>) -> Result<(), String> {
    let wallet_service = wallet.lock().await;
    wallet_service.lock_wallet();
    Ok(())
}

#[tauri::command]
pub async fn delete_wallet(wallet: WalletState<'_>) -> Result<(), String> {
    let wallet_service = wallet.lock().await;
    wallet_service.delete_wallet()
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn is_wallet_initialized(wallet: WalletState<'_>) -> Result<bool, String> {
    let wallet_service = wallet.lock().await;
    Ok(wallet_service.is_initialized())
}

#[tauri::command]
pub async fn is_wallet_unlocked(wallet: WalletState<'_>) -> Result<bool, String> {
    let wallet_service = wallet.lock().await;
    Ok(wallet_service.is_unlocked())
}

// ============================================================================
// Address Management Commands
// ============================================================================

#[tauri::command]
pub async fn get_receiving_address(
    _wallet: WalletState<'_>,
) -> Result<String, String> {
    // This would get the regular Kaspa address
    // For now, return placeholder
    Ok("kaspa:placeholder".to_string())
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MetaAddressResponse {
    pub view_public: String,
    pub spend_public: String,
    pub meta_address_json: String,
}

#[tauri::command]
pub async fn get_meta_address(wallet: WalletState<'_>) -> Result<MetaAddressResponse, String> {
    let wallet_service = wallet.lock().await;
    let data = wallet_service
        .get_meta_address()
        .await
        .map_err(|e| e.to_string())?;

    Ok(MetaAddressResponse {
        view_public: data.view_public,
        spend_public: data.spend_public,
        meta_address_json: data.meta_address_json,
    })
}

#[tauri::command]
pub async fn generate_meta_address(
    wallet: WalletState<'_>,
) -> Result<MetaAddressInfo, String> {
    let wallet_service = wallet.lock().await;
    if !wallet_service.is_unlocked() {
        return Err("Wallet must be unlocked".to_string());
    }
    drop(wallet_service); // Release lock before async operation

    // Generate new meta-address
    use vprogs_transaction_runtime::privacy::StealthKeypair;
    let keypair = StealthKeypair::new();

    Ok(MetaAddressInfo {
        view_public: hex::encode(keypair.view_public.serialize()),
        spend_public: hex::encode(keypair.spend_public.serialize()),
        meta_address: serde_json::json!({
            "view_public": hex::encode(keypair.view_public.serialize()),
            "spend_public": hex::encode(keypair.spend_public.serialize()),
        })
        .to_string(),
    })
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MetaAddressInfo {
    pub view_public: String,
    pub spend_public: String,
    pub meta_address: String,
}

// ============================================================================
// Balance & Transactions Commands
// ============================================================================

#[tauri::command]
pub async fn get_balance(wallet: WalletState<'_>) -> Result<BalanceInfo, String> {
    let wallet_service = wallet.lock().await;
    let balance = wallet_service
        .get_balance()
        .await
        .map_err(|e| e.to_string())?;

    Ok(BalanceInfo {
        total: balance,
        available: balance,
        pending: 0,
        formatted: format!("{:.8} KAS", balance as f64 / 100_000_000.0),
    })
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BalanceInfo {
    pub total: u64,
    pub available: u64,
    pub pending: u64,
    pub formatted: String,
}

#[tauri::command]
pub async fn get_transactions(
    wallet: WalletState<'_>,
) -> Result<Vec<TransactionInfo>, String> {
    let wallet_service = wallet.lock().await;
    wallet_service
        .get_transactions()
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn scan_blockchain(wallet: WalletState<'_>) -> Result<u64, String> {
    // Trigger blockchain scan and return number of transactions found
    let wallet_service = wallet.lock().await;
    wallet_service
        .get_balance()
        .await
        .map_err(|e| e.to_string())
        .map(|_| 0) // Placeholder - would return count
}

// ============================================================================
// Send Transaction Commands
// ============================================================================

#[derive(Debug, Serialize, Deserialize)]
pub struct SendTransactionRequest {
    pub to_address: String,
    pub amount: u64,
    pub privacy_mode: String, // "public", "confidential", "full_privacy"
    pub from_address: String,
}

#[tauri::command]
pub async fn send_transaction(
    request: SendTransactionRequest,
    wallet: WalletState<'_>,
) -> Result<SendTransactionResponse, String> {
    let wallet_service = wallet.lock().await;
    let tx_id = wallet_service
        .send_transaction(
            request.to_address,
            request.amount,
            request.privacy_mode,
            request.from_address,
        )
        .await
        .map_err(|e| e.to_string())?;

    Ok(SendTransactionResponse {
        transaction_id: tx_id.clone(),
        explorer_url: format!("https://explorer.kaspa.org/transactions/{}", tx_id),
    })
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SendTransactionResponse {
    pub transaction_id: String,
    pub explorer_url: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EstimateFeeRequest {
    pub to_address: String,
    pub amount: u64,
    pub privacy_mode: String,
}

#[tauri::command]
pub async fn estimate_fee(
    request: EstimateFeeRequest,
    _wallet: WalletState<'_>,
) -> Result<FeeEstimate, String> {
    // Estimate transaction fee based on privacy mode and amount
    let base_fee = 40_000u64; // Base fee in sompi
    let privacy_multiplier = match request.privacy_mode.as_str() {
        "public" => 1.0,
        "confidential" => 1.5,
        "full_privacy" => 2.0,
        _ => 1.0,
    };

    let estimated_fee = (base_fee as f64 * privacy_multiplier) as u64;

    Ok(FeeEstimate {
        fee: estimated_fee,
        formatted: format!("{:.8} KAS", estimated_fee as f64 / 100_000_000.0),
    })
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FeeEstimate {
    pub fee: u64,
    pub formatted: String,
}

// ============================================================================
// Settings Commands
// ============================================================================

type SettingsState<'a> = State<'a, Mutex<SettingsManager>>;

#[tauri::command]
pub async fn get_settings(settings_manager: SettingsState<'_>) -> Result<Settings, String> {
    let manager = settings_manager.lock().await;
    manager.load()
}

#[tauri::command]
pub async fn update_settings(
    new_settings: Settings,
    settings_manager: SettingsState<'_>,
) -> Result<(), String> {
    let manager = settings_manager.lock().await;
    manager.save(&new_settings)
}

// ============================================================================
// Backup & Restore Commands
// ============================================================================

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateBackupRequest {
    pub password: String,
}

#[tauri::command]
pub async fn create_backup(
    _request: CreateBackupRequest,
    wallet: WalletState<'_>,
) -> Result<String, String> {
    let wallet_service = wallet.lock().await;
    if !wallet_service.is_unlocked() {
        return Err("Wallet must be unlocked".to_string());
    }

    // Create backup
    // This would use BackupManager
    Ok("backup_file_path".to_string())
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RestoreBackupRequest {
    pub backup_path: String,
    pub password: String,
}

#[tauri::command]
pub async fn restore_backup(
    request: RestoreBackupRequest,
    wallet: WalletState<'_>,
) -> Result<(), String> {
    // Restore from backup
    let _ = (request, wallet);
    Ok(())
}

#[tauri::command]
pub async fn list_backups() -> Result<Vec<String>, String> {
    // List available backup files
    // TODO: Implement backup listing
    Ok(vec![])
}

