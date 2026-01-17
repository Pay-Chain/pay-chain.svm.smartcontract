# Pay-Chain Solana Programs

Cross-chain payment programs for Solana built with Rust and Anchor.

## Tech Stack

- **Language**: Rust
- **Framework**: Anchor
- **Testing**: Anchor test framework

## Getting Started

```bash
# Install Anchor CLI
cargo install --git https://github.com/coral-xyz/anchor anchor-cli

# Build programs
anchor build

# Run tests
anchor test

# Deploy to devnet
anchor deploy --provider.cluster devnet
```

## Programs

- `pay_chain` - Main payment gateway program

## Supported Networks (Phase 1)

| Network | Type |
|---------|------|
| Solana Devnet | Testnet |

## Environment Variables

Configure `Anchor.toml`:

```toml
[provider]
cluster = "devnet"
wallet = "~/.config/solana/id.json"
```
