#!/usr/bin/env bash
# Simple deployment helper for Contract Fox smart contracts.
# Usage: ./scripts/deploy.sh [network]
# Defaults to "testnet" when no network argument is provided.

set -euo pipefail

if ! command -v soroban >/dev/null 2>&1; then
    echo "Error: soroban CLI not found. Install it with 'cargo install soroban-cli' or 'npm install -g soroban-cli'." >&2
    exit 1
fi

NETWORK=${1:-testnet}

# ensure profile exists (some soroban versions use "network" subcommand)
if ! soroban network ls | grep -q "^$NETWORK" 2>/dev/null; then
    echo "Network profile '$NETWORK' not found, adding default values."
    case "$NETWORK" in
        testnet)
            soroban network add testnet \
                --rpc-url https://rpc.testnet.soroban.stellar.org \
                --network-passphrase "Test SDF Network ; September 2015"
            ;;
        mainnet)
            soroban network add mainnet \
                --rpc-url https://rpc.mainnet.soroban.stellar.org \
                --network-passphrase "Public Global Stellar Network ; September 2015"
            ;;
        *)
            echo "Warning: no automatic configuration for network '$NETWORK'." >&2
            ;;
    esac
fi


# ensure the wasm target is available
rustup target add wasm32-unknown-unknown >/dev/null 2>&1 || true

# build the contracts using the Makefile helper for consistency
echo "Building WASM contracts..."
make build-contracts

CONTRACT_NAMES=("donation-contract" "withdrawal-contract" "campaign-contract")

for name in "${CONTRACT_NAMES[@]}"; do
    wasm_file="target/wasm32-unknown-unknown/release/${name}.wasm"
    if [ ! -f "$wasm_file" ]; then
        echo "WARNING: $wasm_file not found, skipping."
        continue
    fi
    echo "Deploying $name to $NETWORK..."
    # soroban outputs a line like "Contract ID: GC..."
    soroban contract deploy --wasm "$wasm_file" --network "$NETWORK"
    echo
done

echo "Deployment script finished."
