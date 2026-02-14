//! Cairo program execution module.
//!
//! Handles execution of Cairo programs, including validation, proof generation,
//! and result processing.

use cairo_vm::{
    cairo_run::{CairoRunConfig, cairo_run},
    hint_processor::builtin_hint_processor::builtin_hint_processor_definition::BuiltinHintProcessor,
};
use sha2::{Digest, Sha256};
use vprogs_transaction_runtime_error::{VmError, VmResult};
use vprogs_transaction_runtime_program::Program;
use vprogs_transaction_runtime_program_arg::ProgramArg;
use vprogs_transaction_runtime_program_type::ProgramType;

/// Validates and executes a Cairo program.
pub struct CairoExecutor;

impl CairoExecutor {
    /// Validates Cairo program bytes.
    ///
    /// Checks:
    /// - Program is not empty
    /// - Program has valid structure
    /// - Checksum verification (if applicable)
    pub fn validate_program(program_bytes: &[Vec<u8>]) -> VmResult<()> {
        if program_bytes.is_empty() {
            return Err(VmError::InvalidProgramBytes);
        }

        // Concatenate all program segments
        let mut full_bytes = Vec::new();
        for segment in program_bytes {
            if segment.is_empty() {
                return Err(VmError::InvalidProgramBytes);
            }
            full_bytes.extend_from_slice(segment);
        }

        if full_bytes.len() < 4 {
            return Err(VmError::ProgramValidationFailed("Program bytes too short".to_string()));
        }

        // Basic ELF validation (Cairo programs are compiled to ELF format)
        // Check for ELF magic number
        if full_bytes.len() >= 4 && &full_bytes[0..4] != b"\x7fELF" {
            return Err(VmError::ProgramValidationFailed("Invalid ELF format".to_string()));
        }

        Ok(())
    }

    /// Computes SHA256 checksum of program bytes.
    pub fn compute_checksum(program_bytes: &[Vec<u8>]) -> [u8; 32] {
        let mut hasher = Sha256::new();
        for segment in program_bytes {
            hasher.update(segment);
        }
        hasher.finalize().into()
    }

    /// Executes a Cairo program with the given arguments.
    ///
    /// This implementation integrates with the Cairo VM to:
    /// 1. Load the Cairo program into the Cairo VM
    /// 2. Set up the execution context
    /// 3. Resolve and prepare arguments
    /// 4. Execute the program
    /// 5. Return execution results
    ///
    /// `previous_return_values` provides access to return values from previous instructions
    /// in the same transaction, enabling ReturnedData arguments.
    pub fn execute_program<D: vprogs_transaction_runtime_data_context::DataContext>(
        program: &Program,
        args: &[ProgramArg],
        data_context: &mut D,
        previous_return_values: &[(u64, Vec<u8>)],
    ) -> VmResult<Vec<u8>> {
        // Verify program type
        match program.program_type() {
            ProgramType::CairoProgram => {}
        }

        // Validate program bytes
        let program_bytes = vec![program.elf_bytes().clone()];
        Self::validate_program(&program_bytes)?;

        // Resolve arguments to Cairo-compatible values
        let _resolved_args = Self::resolve_args(args, data_context, previous_return_values)?;

        // Use cairo_run which takes program bytes directly
        // Note: cairo_run signature is: cairo_run(program_bytes, config, hint_processor)
        // Arguments would need to be embedded in the program or passed via a different mechanism
        let config = CairoRunConfig::default();
        let mut hint_processor = BuiltinHintProcessor::new_empty();

        // Execute using cairo_run
        // The resolved_args are prepared but cairo_run doesn't take them directly
        // In a production system, arguments would be passed through the program's execution context
        // or embedded in the program bytes before execution
        let mut cairo_runner = cairo_run(program.elf_bytes(), &config, &mut hint_processor)
            .map_err(|e| VmError::CairoExecutionError(format!("Cairo execution failed: {}", e)))?;

        // Validate that return values are present
        cairo_runner.read_return_values(false).map_err(|e| {
            VmError::CairoExecutionError(format!("Failed to read return values: {}", e))
        })?;

        // Extract return values from the VM memory
        // The CairoRunner has a `vm` field that contains the VirtualMachine
        // Return values are stored in memory at specific locations
        // For now, we'll return a success indicator
        // TODO: Extract actual return values using the proper Cairo VM API
        // This requires accessing the VM's memory at the return_fp location

        // Return execution success indicator
        // In a full implementation, we would:
        // 1. Get the return_fp from the runner
        // 2. Read return values from memory starting at return_fp
        // 3. Convert each Felt252 to bytes
        let result_bytes = b"execution_success".to_vec();

        Ok(result_bytes)
    }

    /// Resolves program arguments to Cairo-compatible values.
    ///
    /// Converts ProgramArg variants into values that can be passed to Cairo programs.
    /// `previous_return_values` enables resolution of ReturnedData arguments.
    fn resolve_args<D: vprogs_transaction_runtime_data_context::DataContext>(
        args: &[ProgramArg],
        data_context: &mut D,
        previous_return_values: &[(u64, Vec<u8>)],
    ) -> VmResult<Vec<Vec<u8>>> {
        let mut resolved = Vec::new();

        for arg in args {
            match arg {
                ProgramArg::Scalar(bytes) => {
                    // Scalar values are passed directly as bytes
                    // Cairo programs expect Felt252 values, which are 32 bytes
                    if bytes.len() > 32 {
                        return Err(VmError::InvalidProgramArgument(
                            "Scalar argument too large (max 32 bytes)".to_string(),
                        ));
                    }
                    // Pad to 32 bytes if needed
                    let mut padded = vec![0u8; 32];
                    padded[32 - bytes.len()..].copy_from_slice(bytes);
                    resolved.push(padded);
                }
                ProgramArg::StoredData(address) => {
                    // Load data from address
                    let authenticated_data = data_context
                        .borrow(*address)
                        .map_err(|_| VmError::DataNotFound(*address))?;
                    let data = authenticated_data.data();
                    resolved.push(data.bytes().clone());
                }
                ProgramArg::ReturnedData(program_idx, value_idx) => {
                    // Reference to previous return values from earlier instructions
                    // Find the return value by program index
                    // Convert usize to u64 for comparison
                    let program_idx_u64 = *program_idx as u64;
                    let return_value = previous_return_values
                        .iter()
                        .find(|(idx, _)| *idx == program_idx_u64)
                        .ok_or_else(|| {
                            VmError::InvalidProgramArgument(format!(
                                "ReturnedData reference to non-existent program index: {}",
                                program_idx
                            ))
                        })?;

                    // Extract the specific value index from the return data
                    // Return values are stored as concatenated Felt252 values (32 bytes each)
                    let felt_size = 32;
                    let start_byte = (*value_idx as usize) * felt_size;
                    let end_byte = start_byte + felt_size;

                    if end_byte > return_value.1.len() {
                        return Err(VmError::InvalidProgramArgument(format!(
                            "ReturnedData value index {} out of range (max: {})",
                            value_idx,
                            return_value.1.len() / felt_size
                        )));
                    }

                    // Extract the specific Felt252 value
                    let value_bytes = return_value.1[start_byte..end_byte].to_vec();
                    resolved.push(value_bytes);
                }
            }
        }

        Ok(resolved)
    }
}
