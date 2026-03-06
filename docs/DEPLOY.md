# Deployment Guide

This document covers the steps required to deploy the Soroban smart contracts contained
in this workspace to the **testnet** or **mainnet** networks. It also provides
examples of invoking deployed contracts using the `soroban` CLI.

---

## 1. Prerequisites

- Rust toolchain with `wasm32-unknown-unknown` target installed.
- [Soroban CLI](https://github.com/stellar/soroban-cli) installed and in your `PATH`.
  You can install via `cargo install soroban-cli` or `npm install -g soroban-cli`.
- (Optional) [`jq`](https://stedolan.github.io/jq/) for parsing JSON output.

---

## 2. Soroban Network Configuration

The workspace ships with a default configuration at `.soroban/config/config.toml`
that already defines the two public networks. You may still add them manually or
create new profiles:

```bash
# add networks once (these commands update your local soroban config file)
# you only need to run them if you want to modify the RPC URL or passphrase
soroban network add testnet \
    --rpc-url https://rpc.testnet.soroban.stellar.org \
    --network-passphrase "Test SDF Network ; September 2015"

soroban network add mainnet \
    --rpc-url https://rpc.mainnet.soroban.stellar.org \
    --network-passphrase "Public Global Stellar Network ; September 2015"

# view available profiles
soroban network ls

# choose an active profile (or set SOROBAN_NETWORK environment variable)
soroban network use testnet
```

> **Note:** the `SOROBAN_RPC_URL` and `SOROBAN_NETWORK_PASSPHRASE` environment
> variables will override whatever values the profile defines. See
> `.env.example` for a template.

---

## 3. Deploying Contracts

A helper script `./scripts/deploy.sh` wraps the common steps:

```sh
# make the script executable if needed
chmod +x ./scripts/deploy.sh

# deploy to testnet (default)
./scripts/deploy.sh testnet

# deploy to mainnet
./scripts/deploy.sh mainnet
```

When executed, the script builds each contract, deploys it to the requested
network, and prints the contract ID returned by the Soroban CLI.

Example output:

```
Building WASM contracts...
Building donation-contract...
Building withdrawal-contract...
Building campaign-contract...
Deploying donation-contract to testnet...
Contract ID: GD5G...7XES
Deploying withdrawal-contract to testnet...
Contract ID: GCFW...9JQK
Deploying campaign-contract to testnet...
Contract ID: GAZQ...3LPT
Deployment script finished.
```

The IDs may be recorded in your own environment (e.g. `.contract_id`) or used
by other automation.

---

## 4. Invoking a Deployed Contract

Once you have a contract ID you can call its methods using `soroban contract
invoke`:

```bash
# simple invocation with no arguments
soroban contract invoke --id GC... --fn ping --network testnet

# with arguments (two string values)
soroban contract invoke --id GC... --fn transfer --network testnet \
    --args GABC... PCDE...
```

The same `--network` flag may be replaced by setting `SOROBAN_NETWORK` or by
using `soroban network use` to change the active profile.

---

## 5. Environment Variables

Copy `.env.example` to `.env` and adjust values as necessary. Relevant variables
for deployment include:

```text
SOROBAN_NETWORK=testnet
SOROBAN_RPC_URL=https://rpc.testnet.soroban.stellar.org
SOROBAN_NETWORK_PASSPHRASE="Test SDF Network ; September 2015"
```

Other tooling (friendbot, CLI helpers) may read these values as well.

---

With the above in place you can confidently build, deploy, and interact with the
contracts on either public Soroban network.
