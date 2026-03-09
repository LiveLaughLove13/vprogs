# Cairo Privacy + Wallet Integration

This is the canonical document for Cairo-based privacy, wallet integration, and current implementation status in this repository.

## Scope

This document covers:

- Cairo execution and proof embedding flow
- Privacy transaction modes (public, confidential, full privacy)
- Wallet and GUI integration status
- Transaction submitter examples
- What is completed vs. what is still in progress

## High-Level Architecture

1. Program logic executes off-chain (Cairo VM).
2. Execution artifacts are converted into transaction effects.
3. A Kaspa transaction is built from those effects.
4. Privacy mode determines how much metadata is hidden.
5. The transaction is signed and submitted through RPC.
6. Wallet/UI layers provide user-facing send/receive workflows.

## Privacy Modes

### Public (`public_transaction_with_cairo_proof`)

- Nothing is hidden.
- Useful for baseline testing and proof-embedding demos.

### Confidential (`confidential_transaction`)

- Amounts are hidden via confidential commitments.
- Sender and recipient addresses remain visible.

### Full Privacy (`full_privacy_transaction`)

- Amounts are hidden.
- Recipient identity is protected using stealth addressing.
- Highest privacy of the three supported modes.

## Integration Points

### Transaction Submitter

`node/transaction-submitter` is responsible for:

- preparing transactions from runtime effects
- handling privacy-mode-specific construction paths
- submitting transactions to Kaspa RPC

### Runtime + Scheduler + Storage

The privacy transaction path composes with the layered architecture:

- `transaction-runtime/` for program and execution semantics
- `scheduling/` for orchestration and batch execution
- `state/` + `storage/` for persistence and rollback-safe state flow

### Wallet GUI

`wallet-gui/` provides:

- wallet init/unlock/lock flows
- send/receive UX
- privacy mode selection in sending flows
- transaction and balance views

## Current Advancement Snapshot

### Completed

- Workspace integration with upstream architecture and crates
- Buildable workspace on Windows (`cargo check` and test compile pass)
- Canonical privacy examples under `node/transaction-submitter/examples/`
- Wallet GUI scaffolding and privacy mode UX path

### In Progress

- Full production transaction building edge cases
- Additional end-to-end wallet UX hardening
- Further polish around operator-facing docs and troubleshooting depth

## Example Commands

```bash
cargo run --example public_transaction_with_cairo_proof --package vprogs-node-transaction-submitter
cargo run --example confidential_transaction --package vprogs-node-transaction-submitter
cargo run --example full_privacy_transaction --package vprogs-node-transaction-submitter
cargo run --example generate_meta_address --package vprogs-node-transaction-submitter
cargo run --example spend_from_stealth_address --package vprogs-node-transaction-submitter
```

## Environment

Typical variables used by examples and wallet flows:

```env
KASPA_RPC_URL=ws://127.0.0.1:17110
FROM_ADDRESS=kaspa:your_sender_address
TO_ADDRESS=kaspa:recipient_or_meta_address
PRIVATE_KEY=your_64_hex_private_key
AMOUNT=100000000
```

## Source of Truth

Use this file as the primary integration/status document instead of older Cairo- or wallet-status markdown notes.
