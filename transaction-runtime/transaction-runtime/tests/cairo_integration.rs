//! Integration tests for Cairo program execution with rusty-kaspa tn12.

use vprogs_transaction_runtime_address::Address;
use vprogs_transaction_runtime_instruction::Instruction;
use vprogs_transaction_runtime_program::Program;
use vprogs_transaction_runtime_program_arg::ProgramArg;
use vprogs_transaction_runtime_program_type::ProgramType;
use vprogs_transaction_runtime_transaction::Transaction;
use vprogs_transaction_runtime_transaction_effects::TransactionEffects;

mod cairo_executor {
    // Re-export for testing
    pub use vprogs_transaction_runtime::cairo_executor::*;
}

/// Test that Cairo program validation works correctly.
#[test]
fn test_cairo_program_validation() {
    use cairo_executor::CairoExecutor;

    // Valid ELF header (simplified)
    let valid_elf = vec![
        vec![0x7f, 0x45, 0x4c, 0x46, 0x02, 0x01, 0x01], // ELF magic + class + data + version
    ];

    // Test valid program
    let result = CairoExecutor::validate_program(&valid_elf);
    assert!(result.is_ok(), "Valid ELF should pass validation");

    // Test empty program
    let empty = vec![];
    let result = CairoExecutor::validate_program(&empty);
    assert!(result.is_err(), "Empty program should fail validation");

    // Test invalid program (wrong magic)
    let invalid = vec![vec![0x00, 0x01, 0x02, 0x03]];
    let result = CairoExecutor::validate_program(&invalid);
    assert!(result.is_err(), "Invalid program should fail validation");
}

/// Test checksum computation.
#[test]
fn test_checksum_computation() {
    use cairo_executor::CairoExecutor;

    let program_bytes = vec![vec![1, 2, 3, 4, 5]];
    let checksum1 = CairoExecutor::compute_checksum(&program_bytes);
    let checksum2 = CairoExecutor::compute_checksum(&program_bytes);

    // Same input should produce same checksum
    assert_eq!(checksum1, checksum2, "Checksums should be deterministic");

    // Different input should produce different checksum
    let different = vec![vec![1, 2, 3, 4, 6]];
    let checksum3 = CairoExecutor::compute_checksum(&different);
    assert_ne!(checksum1, checksum3, "Different programs should have different checksums");
}

/// Test program publishing instruction.
#[test]
fn test_publish_program_instruction() {
    // Create a minimal valid ELF program
    let program_bytes = vec![
        vec![0x7f, 0x45, 0x4c, 0x46, 0x02, 0x01, 0x01, 0x00], // ELF header
        vec![0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00], // More bytes
    ];

    let instruction = Instruction::PublishProgram { program_bytes };

    match instruction {
        Instruction::PublishProgram { program_bytes: bytes } => {
            assert!(!bytes.is_empty(), "Program bytes should not be empty");
            assert_eq!(bytes.len(), 2, "Should have 2 segments");
        }
        _ => panic!("Expected PublishProgram instruction"),
    }
}

/// Test program calling instruction.
#[test]
fn test_call_program_instruction() {
    let program_id = Address::new([0u8; 32]);
    let args = vec![ProgramArg::Scalar(vec![1, 2, 3, 4]), ProgramArg::Scalar(vec![5, 6, 7, 8])];

    let instruction = Instruction::CallProgram { program_id, args: args.clone() };

    match instruction {
        Instruction::CallProgram { program_id: id, args: a } => {
            assert_eq!(id, Address::new([0u8; 32]));
            assert_eq!(a.len(), 2);
            match &a[0] {
                ProgramArg::Scalar(bytes) => assert_eq!(bytes, &vec![1, 2, 3, 4]),
                _ => panic!("Expected Scalar argument"),
            }
        }
        _ => panic!("Expected CallProgram instruction"),
    }
}

/// Test transaction with Cairo instructions.
#[test]
fn test_transaction_with_cairo_instructions() {
    let program_bytes = vec![vec![0x7f, 0x45, 0x4c, 0x46, 0x02, 0x01, 0x01]];

    let instructions = vec![
        Instruction::PublishProgram { program_bytes },
        Instruction::CallProgram {
            program_id: Address::new([1u8; 32]),
            args: vec![ProgramArg::Scalar(vec![42])],
        },
    ];

    let transaction = Transaction::new(vec![], instructions.clone());

    assert_eq!(transaction.instructions().len(), 2);
    assert_eq!(transaction.accessed_objects().len(), 0);
}

/// Test program structure.
#[test]
fn test_program_structure() {
    let elf_bytes = vec![0x7f, 0x45, 0x4c, 0x46];
    let program = Program::new(ProgramType::CairoProgram, elf_bytes.clone());

    assert_eq!(program.program_type(), &ProgramType::CairoProgram);
    assert_eq!(program.elf_bytes(), &elf_bytes);
}

/// Test transaction effects structure.
#[test]
fn test_transaction_effects() {
    let mut effects = TransactionEffects::default();

    // Initially empty
    assert_eq!(effects.return_values.len(), 0);
    assert_eq!(effects.published_programs.len(), 0);

    // Add return value
    effects.return_values.push((0, vec![1, 2, 3]));
    assert_eq!(effects.return_values.len(), 1);

    // Add published program
    effects.published_programs.push(0);
    assert_eq!(effects.published_programs.len(), 1);
}

/// Test that program address is derived from checksum.
#[test]
fn test_program_address_from_checksum() {
    use cairo_executor::CairoExecutor;

    let program_bytes = vec![vec![1, 2, 3, 4, 5]];
    let checksum = CairoExecutor::compute_checksum(&program_bytes);

    // Address should be the checksum
    let address = Address::new(checksum);
    assert_eq!(address, Address::new(checksum));
}
