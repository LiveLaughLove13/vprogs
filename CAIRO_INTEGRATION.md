# Cairo Integration Guide

This document describes the Cairo VM integration in vprogs.

## Overview

Cairo has been integrated into the vprogs transaction runtime to enable zero-knowledge proof program execution on Kaspa. The integration provides:

- Cairo program validation
- Program execution framework
- Checksum verification
- Program publishing and calling

## Architecture

### Components

1. **Cairo Executor** (`transaction-runtime/src/cairo_executor.rs`)
   - Validates Cairo program bytes
   - Computes program checksums
   - Executes Cairo programs
   - Resolves program arguments

2. **Transaction Runtime** (`transaction-runtime/src/lib.rs`)
   - Integrates Cairo executor
   - Handles `PublishProgram` instruction
   - Handles `CallProgram` instruction
   - Manages program state

3. **Program Types** (`transaction-runtime/program-type/src/lib.rs`)
   - `CairoProgram` - Cairo zero-knowledge programs

## Current Implementation Status

### ✅ Completed

- [x] Updated dependencies to use `tn12` branch of rusty-kaspa
- [x] Added Cairo VM dependencies (`cairo-vm`, `sha2`)
- [x] Created Cairo executor module
- [x] Implemented program validation
- [x] Implemented checksum computation
- [x] Completed `PublishProgram` instruction
- [x] Completed `CallProgram` instruction framework
- [x] Updated error types for Cairo-specific errors
- [x] Updated `TransactionEffects` to include return values

### 🔄 Partial Implementation

- [ ] Full Cairo VM execution (placeholder implemented)
  - Program validation is complete
  - Execution framework is in place
  - Needs actual Cairo VM API integration

### 📝 Next Steps

1. **Complete Cairo VM Integration**
   ```rust
   // In cairo_executor.rs, replace placeholder with:
   use cairo_vm::vm::vm_core::VirtualMachine;
   
   let mut vm = VirtualMachine::new();
   vm.load_program(&program.elf_bytes())?;
   vm.run_with_args(&resolved_args)?;
   let return_values = vm.get_return_values()?;
   ```

2. **Add Proof Generation**
   - Integrate Cairo proof generation
   - Store proofs with program execution results
   - Verify proofs on execution

3. **Enhanced Argument Resolution**
   - Complete `ReturnedData` argument handling
   - Add type checking for arguments
   - Support complex data structures

4. **Program Storage**
   - Persist published programs to storage
   - Implement program versioning
   - Add program metadata

## Usage

### Publishing a Cairo Program

```rust
let program_bytes = vec![elf_bytes];
let instruction = Instruction::PublishProgram { program_bytes };
// Program address is computed from checksum
// Program is stored in loaded_programs
```

### Calling a Cairo Program

```rust
let args = vec![
    ProgramArg::Scalar(value_bytes),
    ProgramArg::StoredData(data_address),
];
let instruction = Instruction::CallProgram {
    program_id: program_address,
    args,
};
// Program executes and returns results in TransactionEffects
```

## Dependencies

- `cairo-vm = "0.13.0"` - Cairo virtual machine
- `sha2 = "0.10"` - Checksum computation
- All Kaspa dependencies use `tn12` branch

## Error Handling

New error types added:
- `ProgramNotFound` - Program not found at address
- `ProgramValidationFailed` - Program validation error
- `ProgramExecutionFailed` - Execution error
- `CairoExecutionError` - Cairo-specific error
- `InvalidProgramArgument` - Invalid argument
- `ChecksumMismatch` - Checksum verification failed

## Testing

To test the integration:

1. Build the project:
   ```bash
   cargo build --release
   ```

2. Run tests:
   ```bash
   cargo test
   ```

3. Test with Cairo programs:
   - Create a Cairo program
   - Compile to ELF format
   - Publish using `PublishProgram`
   - Call using `CallProgram`

## Notes

- The current implementation provides the framework for Cairo execution
- Actual Cairo VM API calls need to be added based on the `cairo-vm` crate API
- Program execution results are stored in `TransactionEffects`
- Checksums are computed using SHA256

## Privacy Features

With Cairo integration, vprogs can now support:
- Zero-knowledge proofs
- Confidential transactions
- Privacy-preserving smart contracts
- Verifiable computation

The framework is ready for privacy solution development once the full Cairo VM integration is complete.

