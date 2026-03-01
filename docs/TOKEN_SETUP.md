# AID Token Setup (Testnet)

This guide creates a Stellar custom asset (default: `AID`) with:

- an **issuing account** (asset issuer)
- a **distribution account** (treasury that receives initial supply)

The implementation for this flow lives in `src/setup/token_setup.rs`.

## Prerequisites

- `stellar` CLI installed and authenticated for testnet usage.
- Network access to Stellar testnet services.

## What the setup code does

`setup_custom_asset` performs the following steps:

1. Generates issuing and distribution keypairs (funded on testnet).
2. Reads both public keys.
3. Creates a trustline from distribution account to `AID:<ISSUER>`.
4. Sends a fixed payment from issuing to distribution, minting the initial supply.

## CLI-level commands executed

The setup module runs these commands under the hood (defaults shown):

```bash
stellar keys generate aid_issuing --network testnet --fund --overwrite
stellar keys generate aid_distribution --network testnet --fund --overwrite
stellar keys address aid_issuing
stellar keys address aid_distribution
stellar tx new change-trust \
  --source-account aid_distribution \
  --network testnet \
  --line AID:<ISSUER_PUBLIC_KEY> \
  --sign --submit
stellar tx new payment \
  --source-account aid_issuing \
  --network testnet \
  --destination <DISTRIBUTION_PUBLIC_KEY> \
  --asset AID \
  --amount 1000000 \
  --sign --submit
```

## Expected outcome

After successful execution:

- The custom asset `AID:<ISSUER_PUBLIC_KEY>` exists on testnet.
- The distribution account trustline is active for the AID asset.
- The distribution account holds the configured initial supply (default `1,000,000` AID).

## Example usage from Rust

```rust
use crate::setup::token_setup::{setup_custom_asset, TokenSetupConfig};

let cfg = TokenSetupConfig::default();
let result = setup_custom_asset(&cfg)?;
println!("Issued {} {} to {}", result.amount_issued, result.asset, result.distribution_public_key);
# Ok::<(), Box<dyn std::error::Error>>(())
```

## Notes

- Issuing account should generally not be used for operational spending outside issuance controls.
- For production, add tighter account controls (authorization flags, multisig, limits) before minting.
