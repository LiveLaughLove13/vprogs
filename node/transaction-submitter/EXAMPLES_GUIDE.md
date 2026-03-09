# Transaction Examples Guide

This guide explains what each example does and when to use it.

## Example Overview

| Example | Privacy Level | What's Hidden | Use Case |
|---------|--------------|---------------|----------|
| `public_transaction_with_cairo_proof` | **None** (Public) | Nothing - all details visible | Testing, proof embedding demo |
| `confidential_transaction` | **Medium** | Transaction amounts | Hide how much you're sending |
| `full_privacy_transaction` | **High** | Amounts + recipient address | Maximum privacy |
| `spend_from_stealth_address` | N/A | N/A | How recipients claim funds |
| `generate_meta_address` | N/A | N/A | Generate receiving address |

## Detailed Examples

### 1. `public_transaction_with_cairo_proof.rs`

**Privacy Level:** None (Public Transaction)

**What it does:**
- Sends a regular Kaspa transaction
- Embeds Cairo ZK-STARK proof in transaction payload
- All transaction details are visible on the blockchain

**What's visible:**
- ✅ Sender address
- ✅ Recipient address  
- ✅ Transaction amount
- ✅ All transaction details
- ✅ Cairo proof data (in payload)

**When to use:**
- Testing transaction submission
- Demonstrating Cairo proof embedding
- Cases where privacy is not needed
- Learning how the system works

**Run it:**
```bash
cargo run --example public_transaction_with_cairo_proof --package vprogs-node-transaction-submitter
```

---

### 2. `confidential_transaction.rs`

**Privacy Level:** Medium (Confidential Amounts)

**What it does:**
- Sends a transaction with **hidden amounts**
- Uses Pedersen commitments to hide transaction values
- Uses range proofs to verify amounts are valid
- Addresses are still visible

**What's visible:**
- ✅ Sender address
- ✅ Recipient address
- ❌ Transaction amount (HIDDEN)
- ❌ Change amount (HIDDEN)

**What's hidden:**
- Transaction amounts (using confidential commitments)
- Change amounts

**When to use:**
- You want to hide how much you're sending
- You don't mind addresses being visible
- Medium privacy requirements

**Run it:**
```bash
cargo run --example confidential_transaction --package vprogs-node-transaction-submitter
```

---

### 3. `full_privacy_transaction.rs`

**Privacy Level:** High (Complete Privacy)

**What it does:**
- Uses **stealth addresses** (one-time addresses for recipient)
- Uses **confidential amounts** (hidden transaction values)
- Uses **range proofs** (verifies amounts without revealing)
- Maximum privacy available

**What's visible:**
- ✅ Sender address (still visible)
- ❌ Recipient address (HIDDEN - uses stealth address)
- ❌ Transaction amount (HIDDEN)
- ❌ Change amount (HIDDEN)

**What's hidden:**
- Recipient's real address (uses one-time stealth address)
- Transaction amounts (confidential commitments)
- Change amounts

**When to use:**
- Maximum privacy needed
- Hide who you're sending to
- Hide how much you're sending
- Complete financial privacy

**Run it:**
```bash
cargo run --example full_privacy_transaction --package vprogs-node-transaction-submitter
```

**Note:** Recipient needs to scan the blockchain to discover their funds.

---

### 4. `spend_from_stealth_address.rs`

**Privacy Level:** N/A (Utility Example)

**What it does:**
- Shows how a recipient can claim funds sent to a stealth address
- Demonstrates key derivation for stealth addresses
- Shows how to spend from a one-time address

**When to use:**
- Learning how recipients access stealth funds
- Understanding the stealth address flow
- Testing recipient-side functionality

**Run it:**
```bash
cargo run --example spend_from_stealth_address --package vprogs-node-transaction-submitter
```

---

### 5. `generate_meta_address.rs`

**Privacy Level:** N/A (Utility Example)

**What it does:**
- Generates a stealth keypair (view + spend keys)
- Creates a meta-address for receiving funds
- Shows what to share publicly vs. keep private

**When to use:**
- Setting up to receive stealth payments
- Understanding meta-addresses
- Generating receiving addresses

**Run it:**
```bash
cargo run --example generate_meta_address --package vprogs-node-transaction-submitter
```

---

## Privacy Comparison

### Public Transaction
```
Everyone sees:
- From: kaspa:abc123...
- To: kaspa:def456...
- Amount: 1.0 KAS
- All details
```

### Confidential Transaction
```
Everyone sees:
- From: kaspa:abc123...
- To: kaspa:def456...
- Amount: [HIDDEN]
- Proof that amount is valid
```

### Full Privacy Transaction
```
Everyone sees:
- From: kaspa:abc123...
- To: [ONE-TIME STEALTH ADDRESS]
- Amount: [HIDDEN]
- Proof that amount is valid
- Cannot link to recipient's real address
```

## Quick Decision Guide

**Choose based on your needs:**

1. **Just testing?** → `public_transaction_with_cairo_proof`
2. **Want to hide amounts?** → `confidential_transaction`
3. **Want maximum privacy?** → `full_privacy_transaction`
4. **Want to receive stealth funds?** → `generate_meta_address`
5. **Want to spend stealth funds?** → `spend_from_stealth_address`

## Environment Variables

All examples use the same `.env` file structure:

```env
KASPA_RPC_URL=ws://127.0.0.1:17110
FROM_ADDRESS=kaspa:your_sender_address
PRIVATE_KEY=your_64_hex_character_private_key
TO_ADDRESS=kaspa:recipient_address
AMOUNT=100000000  # in sompi (1 KAS = 100,000,000 sompi)
```

**Note:** For `full_privacy_transaction`, `TO_ADDRESS` should be a meta-address (not a regular Kaspa address).

## Security Reminders

⚠️ **Never share your private keys**  
⚠️ **Test with small amounts first**  
⚠️ **Keep backups of your keys**  
⚠️ **Verify transactions on the explorer**

## Troubleshooting

For integration status and current recommended flows, see:

- `../../CAIRO_PRIVACY_WALLET_INTEGRATION.md`

## Summary

- **`public_transaction_with_cairo_proof`** = No privacy, just proof embedding
- **`confidential_transaction`** = Hide amounts, addresses visible
- **`full_privacy_transaction`** = Hide amounts + recipient address
- **`spend_from_stealth_address`** = How to claim stealth funds
- **`generate_meta_address`** = How to create receiving address

Choose the example that matches your privacy needs!

