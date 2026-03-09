//! Example: Generate Meta-Address for Stealth Addresses
//!
//! This example demonstrates how to generate a stealth keypair and meta-address
//! that can be used to receive funds via stealth addresses.
//!
//! Usage:
//!   cargo run --example generate_meta_address --package vprogs-node-transaction-submitter
//!
//! The meta-address consists of:
//!   - view_public: Public key for scanning transactions
//!   - spend_public: Public key for generating stealth addresses
//!
//! These two public keys are what you publish as your "meta-address"
//! so others can send you funds using stealth addresses.

use vprogs_transaction_runtime::privacy::StealthKeypair;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Generate Meta-Address for Stealth Addresses ===\n");

    // ===================================================================
    // STEP 1: Generate Stealth Keypair
    // ===================================================================
    println!("STEP 1: Generating stealth keypair...");
    println!("─────────────────────────────────────────────────────────");

    let stealth_keys = StealthKeypair::new();

    println!("   ✓ Stealth keypair generated\n");

    // ===================================================================
    // STEP 2: Display Private Keys (KEEP SECRET!)
    // ===================================================================
    println!("STEP 2: Your Private Keys (KEEP THESE SECRET!)");
    println!("─────────────────────────────────────────────────────────");
    println!("   ⚠️  WARNING: Never share these keys with anyone!");
    println!("   ⚠️  Store them securely - you need them to access your funds!\n");

    println!("   View Secret (for scanning transactions):");
    println!("   {}", hex::encode(stealth_keys.view_secret.secret_bytes()));
    println!();

    println!("   Spend Secret (for spending funds):");
    println!("   {}", hex::encode(stealth_keys.spend_secret.secret_bytes()));
    println!();

    // ===================================================================
    // STEP 3: Display Meta-Address (PUBLISH THIS)
    // ===================================================================
    println!("STEP 3: Your Meta-Address (PUBLISH THIS PUBLICLY)");
    println!("─────────────────────────────────────────────────────────");
    println!("   ✅ This is what you share with others");
    println!("   ✅ They use this to send you funds via stealth addresses");
    println!("   ✅ Safe to publish publicly\n");

    println!("   View Public Key:");
    println!("   {}", hex::encode(stealth_keys.view_public.serialize()));
    println!();

    println!("   Spend Public Key:");
    println!("   {}", hex::encode(stealth_keys.spend_public.serialize()));
    println!();

    // ===================================================================
    // STEP 4: Display Meta-Address in Different Formats
    // ===================================================================
    println!("STEP 4: Meta-Address Formats");
    println!("─────────────────────────────────────────────────────────");

    // Format 1: JSON
    println!("   Format 1: JSON");
    println!("   {{");
    println!("     \"view_public\": \"{}\",", hex::encode(stealth_keys.view_public.serialize()));
    println!("     \"spend_public\": \"{}\"", hex::encode(stealth_keys.spend_public.serialize()));
    println!("   }}");
    println!();

    // Format 2: Simple text
    println!("   Format 2: Simple Text");
    println!("   View Public:  {}", hex::encode(stealth_keys.view_public.serialize()));
    println!("   Spend Public: {}", hex::encode(stealth_keys.spend_public.serialize()));
    println!();

    // Format 3: Single line (for easy copying)
    println!("   Format 3: Single Line (for .env file)");
    println!("   META_ADDRESS_VIEW_PUBLIC={}", hex::encode(stealth_keys.view_public.serialize()));
    println!("   META_ADDRESS_SPEND_PUBLIC={}", hex::encode(stealth_keys.spend_public.serialize()));
    println!();

    // ===================================================================
    // STEP 5: Instructions
    // ===================================================================
    println!("STEP 5: What to Do Next");
    println!("─────────────────────────────────────────────────────────");
    println!("   1. ✅ SAVE YOUR PRIVATE KEYS SECURELY");
    println!("      - View Secret: {}", hex::encode(stealth_keys.view_secret.secret_bytes()));
    println!("      - Spend Secret: {}", hex::encode(stealth_keys.spend_secret.secret_bytes()));
    println!();
    println!("   2. ✅ PUBLISH YOUR META-ADDRESS");
    println!("      - View Public:  {}", hex::encode(stealth_keys.view_public.serialize()));
    println!("      - Spend Public: {}", hex::encode(stealth_keys.spend_public.serialize()));
    println!();
    println!("   3. ✅ OTHERS CAN NOW SEND YOU FUNDS");
    println!("      - They use your meta-address to generate stealth addresses");
    println!("      - Each transaction uses a unique one-time address");
    println!("      - Only you can derive the private key to spend");
    println!();
    println!("   4. ✅ TO RECEIVE FUNDS");
    println!("      - Scan the blockchain for transactions");
    println!("      - Derive stealth addresses using your private keys");
    println!("      - When you find a match, derive the private key");
    println!("      - Spend the funds using the derived private key");
    println!();

    // ===================================================================
    // STEP 6: Example .env File
    // ===================================================================
    println!("STEP 6: Example .env File");
    println!("─────────────────────────────────────────────────────────");
    println!("   Create a .env file with:");
    println!();
    println!("   # Your stealth keys (KEEP SECRET!)");
    println!("   STEALTH_VIEW_SECRET={}", hex::encode(stealth_keys.view_secret.secret_bytes()));
    println!("   STEALTH_SPEND_SECRET={}", hex::encode(stealth_keys.spend_secret.secret_bytes()));
    println!();
    println!("   # Your meta-address (PUBLISH THIS)");
    println!("   META_ADDRESS_VIEW_PUBLIC={}", hex::encode(stealth_keys.view_public.serialize()));
    println!("   META_ADDRESS_SPEND_PUBLIC={}", hex::encode(stealth_keys.spend_public.serialize()));
    println!();

    // ===================================================================
    // SUMMARY
    // ===================================================================
    println!("=== Summary ===");
    println!();
    println!("✅ Stealth keypair generated");
    println!("✅ Meta-address ready to publish");
    println!("✅ Private keys ready to store securely");
    println!();
    println!("📝 Next Steps:");
    println!("   1. Save your private keys securely");
    println!("   2. Publish your meta-address (view_public + spend_public)");
    println!("   3. Others can now send you funds using stealth addresses");
    println!("   4. You can scan and claim funds using your private keys");
    println!();
    println!("🔐 Security Reminders:");
    println!("   - Never share your private keys");
    println!("   - Store private keys securely (encrypted, hardware wallet, etc.)");
    println!("   - Meta-address is safe to publish publicly");
    println!("   - Each transaction uses a unique stealth address");
    println!("   - Only you can derive the private key to spend");

    Ok(())
}
