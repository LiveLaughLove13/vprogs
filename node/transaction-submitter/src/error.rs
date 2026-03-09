use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("Transaction submission error: {0}")]
    Submission(String),

    #[error("Connection error: {0}")]
    Connection(String),

    #[error("Not implemented: {0}")]
    NotImplemented(String),

    #[error("RPC error: {0}")]
    Rpc(String),
}

pub type Result<T> = std::result::Result<T, Error>;
