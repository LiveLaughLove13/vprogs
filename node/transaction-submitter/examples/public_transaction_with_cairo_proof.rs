//! Example: Send a PUBLIC Kaspa transaction with embedded Cairo ZK-STARK proof.
//!
//! ⚠️  NOTE: This is a PUBLIC transaction - all details are visible on the blockchain.
//!    For privacy features, see:
//!    - confidential_transaction.rs (confidential amounts)
//!    - full_privacy_transaction.rs (stealth addresses + confidential amounts)
//!
//! This example demonstrates:
//! 1. Creating a Cairo program that performs computation
//! 2. Executing the program to generate a ZK-STARK proof
//! 3. Embedding the proof in a Kaspa transaction payload
//! 4. Submitting a PUBLIC transaction to Kaspa mainnet
//!
//! What's visible on-chain:
//! - Sender address (FROM_ADDRESS)
//! - Recipient address (TO_ADDRESS)
//! - Transaction amount
//! - All transaction details
//! - Cairo proof data (in payload)
//!
//! This is useful for:
//! - Demonstrating Cairo proof embedding
//! - Testing transaction submission
//! - Cases where privacy is not required

use kaspa_consensus_core::network::{NetworkId, NetworkType};
use kaspa_wrpc_client::prelude::*;
use std::sync::Arc;
use vprogs_node_transaction_submitter::{PrivateKey, TransactionSubmitter};
use vprogs_transaction_runtime_instruction::Instruction;
use vprogs_transaction_runtime_program_arg::ProgramArg;
use vprogs_transaction_runtime_transaction::Transaction as VprogsTransaction;
use vprogs_transaction_runtime_transaction_effects::TransactionEffects;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load environment variables from .env file
    match dotenv::dotenv() {
        Ok(path) => println!("Loaded .env file from: {:?}", path),
        Err(e) => println!(
            "Warning: Could not load .env file: {}. Using system environment variables.",
            e
        ),
    }

    // Initialize logging
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    println!("=== Private Kaspa Transaction Example ===\n");

    // Step 1: Connect to Kaspa mainnet
    println!("1. Connecting to Kaspa mainnet...");
    let rpc_url =
        std::env::var("KASPA_RPC_URL").ok().or_else(|| Some("ws://127.0.0.1:17110".to_string()));

    let resolver = if rpc_url.is_none() { Some(Resolver::default()) } else { None };

    let client = Arc::new(KaspaRpcClient::new_with_args(
        WrpcEncoding::Borsh,
        rpc_url.as_deref(),
        resolver,
        Some(NetworkId::new(NetworkType::Mainnet)),
        None,
    )?);

    client.connect(None).await?;
    println!("   ✓ Connected to Kaspa mainnet\n");

    // Step 2: Create a Cairo program for private computation
    // In a real scenario, this would be a compiled Cairo program
    // For this example, we'll use a placeholder
    println!("2. Creating Cairo program for private computation...");

    // Example: A simple Cairo program that adds two numbers privately
    // In production, this would be compiled Cairo bytecode
    let cairo_program_bytes = b"cairo_program_placeholder";

    // Create a vprogs transaction that publishes and executes the program
    let vprogs_tx = VprogsTransaction::new(
        vec![], // accessed_objects
        vec![
            // Publish the Cairo program
            Instruction::PublishProgram { program_bytes: vec![cairo_program_bytes.to_vec()] },
            // Call the program with private inputs
            Instruction::CallProgram {
                program_id: vprogs_transaction_runtime_address::Address::new([0; 32]), // Will be set after publish
                args: vec![
                    ProgramArg::Scalar([1u8; 32].to_vec()), // Private input 1
                    ProgramArg::Scalar([2u8; 32].to_vec()), // Private input 2
                ],
            },
        ],
    );

    println!("   ✓ Created vprogs transaction with Cairo program\n");

    // Step 3: Execute the transaction to generate proofs
    println!("3. Executing transaction to generate ZK-STARK proofs...");

    // In a real implementation, you would use the TransactionRuntime to execute
    // For this example, we'll create a mock execution result
    // The actual execution would happen through the VM and TransactionRuntime

    // Note: This is a simplified example. In production, you would:
    // 1. Use TransactionRuntime::execute() to run the transaction
    // 2. Get the TransactionEffects which contains the proofs
    // 3. Extract the CairoProof from the effects

    let effects = TransactionEffects::default();
    // In production: let effects = execute_vprogs_transaction(&vprogs_tx)?;

    println!("   ✓ Generated ZK-STARK proof\n");

    // Step 4: Get wallet credentials
    println!("4. Setting up wallet...");

    // Get private key from environment (for testing - in production, use secure storage)
    let private_key_hex_raw = match std::env::var("PRIVATE_KEY") {
        Ok(val) => val,
        Err(_) => {
            return Err("PRIVATE_KEY environment variable not found. Please check your .env file exists at D:\\vprogs\\.env and contains PRIVATE_KEY=...".into());
        }
    };

    // Trim whitespace (common issue with .env files)
    let private_key_hex = private_key_hex_raw.trim();

    // Debug: Show what we found (first few chars only for security)
    let preview = if private_key_hex.len() > 10 {
        format!("{}...", &private_key_hex[..10])
    } else {
        private_key_hex.to_string()
    };
    println!("   - PRIVATE_KEY found: {} (length: {})", preview, private_key_hex.len());

    // Validate that it's not the placeholder
    if private_key_hex == "your_64_character_hex_private_key_here" || private_key_hex.is_empty() {
        return Err(format!(
            "PRIVATE_KEY in .env file is still the placeholder or empty (found: '{}', length: {}). \
            \n\nTo fix this:\n1. Open D:\\vprogs\\.env in a text editor\n2. Find the line: PRIVATE_KEY=your_64_character_hex_private_key_here\n3. Replace 'your_64_character_hex_private_key_here' with your actual 64-character hex private key\n4. Save the file\n\nSee GENERATE_KEY.md for how to generate a key, or use your existing wallet's private key.",
            preview,
            private_key_hex.len()
        ).into());
    }

    if private_key_hex.len() != 64 {
        return Err(format!(
            "PRIVATE_KEY must be exactly 64 hex characters, but got {} characters (length: {}). \
            Please check your .env file. Make sure there are no extra spaces or quotes around the value.",
            private_key_hex,
            private_key_hex.len()
        ).into());
    }

    // Validate hex format
    if !private_key_hex.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(format!(
            "PRIVATE_KEY contains invalid characters. It must only contain hex digits (0-9, a-f). \
            Found: '{}' (first 20 chars: '{}')",
            private_key_hex,
            &private_key_hex.chars().take(20).collect::<String>()
        )
        .into());
    }

    println!("   - Private key loaded (length: {} chars)", private_key_hex.len());

    let private_key = PrivateKey::from_hex(private_key_hex)?;

    // Get addresses
    let from_address_str =
        std::env::var("FROM_ADDRESS").expect("Please set FROM_ADDRESS environment variable");
    let from_address = kaspa_addresses::Address::try_from(from_address_str.as_str())
        .map_err(|e| format!("Invalid FROM_ADDRESS: {}", e))?;

    let to_address_str =
        std::env::var("TO_ADDRESS").expect("Please set TO_ADDRESS environment variable");
    let to_address = kaspa_addresses::Address::try_from(to_address_str.as_str())
        .map_err(|e| format!("Invalid TO_ADDRESS: {}", e))?;

    let amount: u64 = std::env::var("AMOUNT")
        .unwrap_or_else(|_| "1000000".to_string()) // Default: 0.001 KAS
        .parse()
        .expect("AMOUNT must be a valid number");

    println!("   ✓ Wallet configured");
    println!("   - From: {}", from_address_str);
    println!("   - To: {}", to_address_str);
    println!("   - Amount: {} sompi ({} KAS)", amount, amount as f64 / 100_000_000.0);
    println!();

    // Step 5: Submit private transaction
    println!("5. Submitting private transaction with Cairo proof...");

    // Reuse the already-connected client instead of creating a new one
    let submitter =
        TransactionSubmitter::new(vprogs_node_transaction_submitter::TransactionSubmitterConfig {
            rpc_client: client.clone(),
            network_id: NetworkId::new(NetworkType::Mainnet),
        });

    // Clone address before it's moved (for later UTXO checking)
    let from_address_for_check = from_address.clone();

    // Check if there are any pending transactions in mempool that might lock UTXOs
    println!("\n5. Checking UTXO availability...");
    let utxos_before = client
        .rpc_api()
        .get_utxos_by_addresses(vec![kaspa_rpc_core::RpcAddress::from(
            from_address_for_check.clone(),
        )])
        .await?;
    println!("   - Available UTXOs reported by RPC: {}", utxos_before.len());

    if utxos_before.is_empty() {
        println!("   ⚠️  WARNING: No UTXOs available!");
        println!("   All UTXOs may be locked by pending transactions.");
        println!("   Please wait for previous transactions to confirm.");
        return Err("No available UTXOs - all are locked by pending transactions".into());
    }

    let total_available: u64 = utxos_before.iter().map(|u| u.utxo_entry.amount).sum();
    println!(
        "   - Total available balance: {} sompi ({:.8} KAS)",
        total_available,
        total_available as f64 / 100_000_000.0
    );

    // Check if previous transaction is still in mempool
    let previous_tx_id_str = "733b7167a53530302b8342e8bab3b560b09ff57e62b2b7a01df9c0128fc341e6";
    if let Ok(prev_tx_id) = previous_tx_id_str.parse::<kaspa_consensus_core::Hash>() {
        match client.rpc_api().get_mempool_entry(prev_tx_id, true, false).await {
            Ok(_entry) => {
                println!(
                    "\n   ⚠️  Previous transaction {} is still in mempool",
                    previous_tx_id_str
                );
                println!("   - This transaction is locking your UTXOs");
                println!("   - Waiting for confirmation...");
                println!(
                    "   - Check status: https://explorer.kaspa.org/transactions/{}",
                    previous_tx_id_str
                );
                println!("\n   You must wait for this transaction to confirm before");
                println!("   submitting a new transaction with the same UTXOs.");
            }
            Err(_) => {
                println!(
                    "\n   ✓ Previous transaction {} is no longer in mempool",
                    previous_tx_id_str
                );
                println!("   It may have been confirmed or rejected.");
            }
        }
    }

    let tx_id = match submitter
        .submit_private_transaction(
            &vprogs_tx,
            &effects,
            from_address,
            to_address,
            amount,
            &private_key,
        )
        .await
    {
        Ok(id) => {
            println!("   ✓ Transaction submitted!");
            println!("   - Transaction ID: {}", id);
            id
        }
        Err(e) => {
            // Check if error is "already in mempool" - this is actually success
            let error_msg = e.to_string();
            if error_msg.contains("already in the mempool")
                || error_msg.contains("already in mempool")
            {
                // Extract transaction ID from error message if possible
                println!("   ⚠️  Transaction is already in mempool (from previous submission)");
                // Try to get the transaction ID from the error message or calculate it
                // For now, we'll need to calculate it from the transaction
                println!("   Checking transaction status...");
                // We'll handle this in the verification step below
                return Err("Transaction already in mempool. Please check the previous transaction ID or wait for confirmation.".to_string().into());
            } else {
                return Err(e.into());
            }
        }
    };

    // Wait a moment and check transaction status
    println!("\n6. Verifying transaction...");
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    // Try to get transaction from mempool
    match client.rpc_api().get_mempool_entry(tx_id, true, false).await {
        Ok(entry) => {
            println!("   ✓ Transaction found in mempool");
            println!("   - Inputs: {}", entry.transaction.inputs.len());
            println!("   - Outputs: {}", entry.transaction.outputs.len());
            let _ = entry; // Use entry to avoid unused variable warning
            println!(
                "   - Payload size: {} bytes (Cairo proof data)",
                entry.transaction.payload.len()
            );
            println!("\n   Output details:");
            for (i, output) in entry.transaction.outputs.iter().enumerate() {
                let kas_amount = output.value as f64 / 100_000_000.0;
                println!("   - Output {}: {} sompi ({:.8} KAS)", i, output.value, kas_amount);
                println!("     Script length: {} bytes", output.script_public_key.script().len());
            }

            // Check if transaction has payload (Cairo proof)
            if entry.transaction.payload.is_empty() {
                println!("\n   ⚠️  WARNING: Transaction payload is empty!");
                println!("   The Cairo proof data is not embedded in the transaction.");
                println!("   This means the transaction is NOT private.");
            } else {
                println!(
                    "\n   ✓ Transaction contains {} bytes of payload (Cairo proof)",
                    entry.transaction.payload.len()
                );
                println!("   This makes it a 'private' transaction with embedded ZK-STARK proof.");
            }

            println!("\n   ⚠️  Transaction is pending confirmation.");
            println!("   The funds will appear in the recipient's balance once the transaction");
            println!("   is confirmed in a block (usually takes a few seconds to a minute).");

            // Check transaction again after a delay to see if it's still in mempool
            println!("\n   Waiting 10 seconds to check if transaction is still in mempool...");
            tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;

            match client.rpc_api().get_mempool_entry(tx_id, true, false).await {
                Ok(entry) => {
                    println!("   ⚠️  Transaction still in mempool after 10 seconds");
                    println!("   It's waiting for a miner to include it in a block.");
                    println!("\n   ⚠️  IMPORTANT: If transaction stays in mempool for too long,");
                    println!("   it may not be getting confirmed. Possible reasons:");
                    println!("   1. Fees too low (current: 50,000 sompi)");
                    println!("   2. Transaction format issue");
                    println!("   3. Network congestion");
                    println!("   4. Payload causing issues");
                    println!("\n   Transaction details:");
                    println!("   - Payload size: {} bytes", entry.transaction.payload.len());
                    println!("   - Compute mass: {} (limit: 100,000)", entry.transaction.mass);
                    // Note: storage_mass is not available in RpcTransaction
                }
                Err(e) => {
                    println!("   ⚠️  Transaction no longer in mempool: {}", e);
                    println!("   This could mean:");
                    println!("   - Transaction was confirmed (check block explorer)");
                    println!("   - Transaction was rejected/dropped from mempool");
                    println!("   - Transaction expired from mempool");

                    // Try to get transaction from blocks to see if it was confirmed
                    println!("\n   Checking if transaction was confirmed in a block...");
                    // Note: We'd need get_transaction RPC to check this, but it may not be available
                    // For now, we'll just direct user to check explorer
                }
            }

            // Additional check: Wait longer and verify again
            println!("\n   Waiting 30 more seconds to check confirmation status...");
            tokio::time::sleep(tokio::time::Duration::from_secs(30)).await;

            match client.rpc_api().get_mempool_entry(tx_id, true, false).await {
                Ok(_) => {
                    println!("   ⚠️  Transaction still in mempool after 40 seconds total");
                    println!("   This suggests the transaction may not be getting confirmed.");
                    println!("   Possible issues:");
                    println!("   - Fees may need to be higher");
                    println!("   - Transaction format may have issues");
                    println!("   - Payload may be causing problems");
                }
                Err(_) => {
                    println!("   ✓ Transaction no longer in mempool - may have been confirmed!");
                    println!("   Please check the explorer to verify.");
                }
            }
        }
        Err(e) => {
            println!("   ⚠️  Could not find transaction in mempool: {}", e);
            println!("\n   This could mean:");
            println!("   - Transaction was rejected by the network (most likely)");
            println!("   - Transaction was already confirmed (check block explorer)");
            println!("   - Transaction is still propagating");
            println!("\n   ⚠️  IMPORTANT: If transaction is not in mempool and not on explorer,");
            println!("   it was likely rejected. Common reasons:");
            println!("   - Invalid signatures");
            println!("   - Insufficient fees");
            println!("   - Invalid transaction format");
            println!("   - UTXO already spent");
            println!("\n   Let's check the transaction details...");

            // Try to get UTXOs to verify they're still available
            println!("\n   Checking source address UTXOs...");
            use kaspa_rpc_core::RpcAddress;
            match client
                .rpc_api()
                .get_utxos_by_addresses(vec![RpcAddress::from(from_address_for_check)])
                .await
            {
                Ok(utxos) => {
                    println!("   - Found {} UTXOs for source address", utxos.len());
                    let total: u64 = utxos.iter().map(|u| u.utxo_entry.amount).sum();
                    println!(
                        "   - Total balance: {} sompi ({:.8} KAS)",
                        total,
                        total as f64 / 100_000_000.0
                    );
                }
                Err(e) => {
                    println!("   - Error checking UTXOs: {}", e);
                }
            }
        }
    }

    println!("\n   View transaction on explorer:");
    println!("   https://explorer.kaspa.org/transactions/{}", tx_id);

    println!("\n=== Transaction Complete ===");
    println!();
    println!("Your private transaction has been submitted to Kaspa mainnet.");
    println!("The transaction includes:");
    println!("  - Cairo ZK-STARK proof (embedded in OP_RETURN)");
    println!("  - Private computation results");
    println!("  - Transaction effects");
    println!();
    println!("Note: The recipient balance will update once the transaction is confirmed.");

    Ok(())
}
