use std::collections::{HashMap, HashSet};

use borsh::BorshDeserialize;
use vprogs_core_types::AccessMetadata;
use vprogs_scheduling_scheduler::{AccessHandle, VmInterface};
use vprogs_state_space::StateSpace;
use vprogs_storage_types::Store;
use vprogs_transaction_runtime_address::Address;
use vprogs_transaction_runtime_authenticated_data::AuthenticatedData;
use vprogs_transaction_runtime_data::Data;
use vprogs_transaction_runtime_error::{VmError, VmResult};
use vprogs_transaction_runtime_instruction::Instruction;
use vprogs_transaction_runtime_lock::Lock;
use vprogs_transaction_runtime_object_id::ObjectId;
use vprogs_transaction_runtime_program::Program;
use vprogs_transaction_runtime_program_context::ProgramContext;
use vprogs_transaction_runtime_program_type::ProgramType;
use vprogs_transaction_runtime_pubkey::PubKey;
use vprogs_transaction_runtime_transaction::Transaction;
use vprogs_transaction_runtime_transaction_effects::TransactionEffects;

pub mod cairo_executor;

pub struct TransactionRuntime<'a, 'b, S, V>
where
    S: Store<StateSpace = StateSpace>,
    V: VmInterface<ResourceId = ObjectId>,
{
    handles: &'a mut [AccessHandle<'b, S, V>],
    signers: HashSet<PubKey>,
    loaded_data: HashMap<Address, AuthenticatedData>,
    loaded_programs: HashMap<Address, Program>,
    effects: TransactionEffects,
    return_value_index: u64,
}

impl<'a, 'b, S, V> TransactionRuntime<'a, 'b, S, V>
where
    S: Store<StateSpace = StateSpace>,
    V: VmInterface<ResourceId = ObjectId>,
{
    pub fn execute(
        tx: &'a Transaction,
        handles: &'a mut [AccessHandle<'b, S, V>],
    ) -> VmResult<TransactionEffects> {
        let signers = HashSet::new();
        let loaded_data = HashMap::new();
        let loaded_programs = HashMap::new();
        let effects = TransactionEffects::default();
        let mut this =
            Self { handles, signers, loaded_data, loaded_programs, effects, return_value_index: 0 };

        this.ingest_state()?;

        for instruction in tx.instructions() {
            this.execute_instruction(instruction)?;
        }

        this.finalize()
    }

    fn ingest_state(&mut self) -> VmResult<()> {
        for handle in self.handles.iter() {
            match handle.access_metadata().id() {
                ObjectId::Program(address) => {
                    let program = Program::deserialize(&mut handle.data().as_slice())?;

                    // Validate program based on type
                    match program.program_type() {
                        ProgramType::CairoProgram => {
                            // Validate Cairo program bytes
                            let program_bytes = vec![program.elf_bytes().clone()];
                            cairo_executor::CairoExecutor::validate_program(&program_bytes)?;
                        }
                    }

                    self.loaded_programs.insert(address, program);
                }
                ObjectId::Data(address) => {
                    // Storage format: Lock | Data (serialized sequentially with Borsh)
                    let mut reader = handle.data().as_slice();
                    let lock = Lock::deserialize(&mut reader)?;
                    let data = Data::deserialize(&mut reader)?;
                    let mut_cap = lock.unlock(self);

                    self.loaded_data.insert(address, AuthenticatedData::new(data, mut_cap));
                }
                ObjectId::Empty => {
                    return Err(VmError::Generic);
                }
            }
        }
        Ok(())
    }

    fn execute_instruction(&mut self, instruction: &Instruction) -> VmResult<()> {
        match instruction {
            Instruction::PublishProgram { program_bytes } => {
                // Validate program bytes
                cairo_executor::CairoExecutor::validate_program(program_bytes)?;

                // Compute checksum for verification
                let checksum = cairo_executor::CairoExecutor::compute_checksum(program_bytes);

                // Concatenate program segments
                let mut full_bytes = Vec::new();
                for segment in program_bytes {
                    full_bytes.extend_from_slice(segment);
                }

                // Create program (assuming Cairo for now)
                let program = Program::new(ProgramType::CairoProgram, full_bytes);

                // Generate program address from checksum
                let program_address = Address::new(checksum);

                // Store program in loaded programs (in-memory cache)
                self.loaded_programs.insert(program_address, program.clone());

                // Persist program to storage via handles
                // Programs are stored in the Metadata state space with key = program_address
                // The actual persistence happens through the scheduling layer's commit mechanism
                // For now, we store it in the loaded_programs map which serves as a cache
                // Full persistence will happen when the transaction is committed
                // TODO: Implement actual storage persistence when storage layer integration is complete

                // Add to published programs in effects
                self.effects.published_programs.push(self.return_value_index);
                self.return_value_index = self.return_value_index.saturating_add(1);
            }
            Instruction::CallProgram { program_id, args } => {
                // Extract values to avoid borrow conflicts
                let program_id_val = *program_id;
                let args = args.to_vec(); // Clone args to avoid lifetime issues

                // Get program and clone it - use a helper to ensure borrow is dropped
                let program = Self::get_program_clone(&self.loaded_programs, program_id_val)?;

                // Clone return values before borrowing self mutably
                let previous_returns = self.effects.return_values.clone();

                // Now we can borrow self mutably for ProgramContext
                // The program is fully owned (cloned), so no conflict
                let result = {
                    let mut program_context = ProgramContext::new(self, program_id_val);
                    // Pass previous return values to enable ReturnedData arguments
                    cairo_executor::CairoExecutor::execute_program(
                        &program,
                        &args,
                        &mut program_context,
                        &previous_returns,
                    )?
                };

                // Store return value in effects
                self.effects.return_values.push((self.return_value_index, result.clone()));

                // Generate and store proof (placeholder for now - full proof generation requires Cairo prover)
                let proof = vprogs_transaction_runtime_transaction_effects::CairoProof {
                    proof_bytes: b"proof_placeholder".to_vec(), // TODO: Generate actual ZK-STARK proof
                    program_address: program_id_val.as_bytes(),
                };
                self.effects.proofs.push((self.return_value_index, proof));

                self.return_value_index = self.return_value_index.saturating_add(1);
            }
        }
        Ok(())
    }

    /// Helper function to get and clone a program, avoiding borrow checker issues.
    fn get_program_clone(
        programs: &HashMap<Address, Program>,
        program_id: Address,
    ) -> VmResult<Program> {
        programs
            .get(&program_id)
            .ok_or_else(|| VmError::ProgramNotFound(program_id))
            .map(|p| p.clone())
    }

    fn finalize(self) -> VmResult<TransactionEffects> {
        Ok(self.effects)
    }
}

mod auth_context;
mod data_context;
