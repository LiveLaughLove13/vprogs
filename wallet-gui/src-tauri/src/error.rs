use thiserror::Error;

#[derive(Error, Debug)]
pub enum WalletError {
    #[error("Wallet not initialized")]
    NotInitialized,

    #[error("Wallet is locked")]
    WalletLocked,

    #[error("Invalid password")]
    InvalidPassword,

    #[error("RPC error: {0}")]
    Rpc(String),

    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("Connection error: {0}")]
    Connection(String),

    #[error("Transaction error: {0}")]
    #[allow(dead_code)] // Reserved for future transaction error handling
    Transaction(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Other error: {0}")]
    Other(String),
}

impl From<vprogs_node_transaction_submitter::SubmitError> for WalletError {
    fn from(err: vprogs_node_transaction_submitter::SubmitError) -> Self {
        match err {
            vprogs_node_transaction_submitter::SubmitError::Connection(msg) => {
                WalletError::Connection(msg)
            }
            vprogs_node_transaction_submitter::SubmitError::Serialization(msg) => {
                WalletError::Serialization(msg)
            }
            vprogs_node_transaction_submitter::SubmitError::Rpc(msg) => WalletError::Rpc(msg),
            _ => WalletError::Other(err.to_string()),
        }
    }
}

pub type Result<T> = std::result::Result<T, WalletError>;

