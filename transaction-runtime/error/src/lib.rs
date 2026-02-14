use std::io::Error;

use vprogs_transaction_runtime_address::Address;

#[derive(Debug)]
pub enum VmError {
    Generic,
    DataNotFound(Address),
    MissingMutCapability(Address),
    SerializationError(String),
    ProgramNotFound(Address),
    ProgramValidationFailed(String),
    ProgramExecutionFailed(String),
    InvalidProgramBytes,
    CairoExecutionError(String),
    InvalidProgramArgument(String),
    ChecksumMismatch,
}

impl Clone for VmError {
    fn clone(&self) -> Self {
        match self {
            VmError::Generic => VmError::Generic,
            VmError::DataNotFound(addr) => VmError::DataNotFound(*addr),
            VmError::MissingMutCapability(addr) => VmError::MissingMutCapability(*addr),
            VmError::SerializationError(msg) => VmError::SerializationError(msg.clone()),
            VmError::ProgramNotFound(addr) => VmError::ProgramNotFound(*addr),
            VmError::ProgramValidationFailed(msg) => VmError::ProgramValidationFailed(msg.clone()),
            VmError::ProgramExecutionFailed(msg) => VmError::ProgramExecutionFailed(msg.clone()),
            VmError::InvalidProgramBytes => VmError::InvalidProgramBytes,
            VmError::CairoExecutionError(msg) => VmError::CairoExecutionError(msg.clone()),
            VmError::InvalidProgramArgument(msg) => VmError::InvalidProgramArgument(msg.clone()),
            VmError::ChecksumMismatch => VmError::ChecksumMismatch,
        }
    }
}

impl From<Error> for VmError {
    fn from(err: Error) -> Self {
        VmError::SerializationError(err.to_string())
    }
}

impl std::fmt::Display for VmError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VmError::Generic => write!(f, "Generic VM error"),
            VmError::DataNotFound(addr) => write!(f, "Data not found at address: {:?}", addr),
            VmError::MissingMutCapability(addr) => {
                write!(f, "Missing mutation capability for address: {:?}", addr)
            }
            VmError::SerializationError(e) => write!(f, "Serialization error: {}", e),
            VmError::ProgramNotFound(addr) => write!(f, "Program not found at address: {:?}", addr),
            VmError::ProgramValidationFailed(msg) => {
                write!(f, "Program validation failed: {}", msg)
            }
            VmError::ProgramExecutionFailed(msg) => write!(f, "Program execution failed: {}", msg),
            VmError::InvalidProgramBytes => write!(f, "Invalid program bytes"),
            VmError::CairoExecutionError(msg) => write!(f, "Cairo execution error: {}", msg),
            VmError::InvalidProgramArgument(msg) => write!(f, "Invalid program argument: {}", msg),
            VmError::ChecksumMismatch => write!(f, "Checksum mismatch"),
        }
    }
}

impl std::error::Error for VmError {}

pub type VmResult<T> = Result<T, VmError>;
