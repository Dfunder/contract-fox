### Makefile for Stellar / Soroban contract development
# Usage:
#   make build           # build workspace
#   make wasm            # build contract WASM (package: $(CONTRACT_PKG))
#   make deploy          # deploy WASM using soroban CLI (requires `soroban`)
#   make fund ADDR=G...  # fund an address on testnet using Friendbot (curl)
#   make invoke FUNC=ping # invoke a function on deployed contract (requires CONTRACT_ID)
#   make test            # run cargo test
#   make fmt             # run cargo fmt
#   make lint            # run cargo clippy (strict)
#   make clean           # cargo clean

# --- Configuration ---
CONTRACT_PKG ?= stellaraid-core
WASM_TARGET ?= wasm32-unknown-unknown
RELEASE_FLAG ?= --release
NETWORK ?= testnet
WASM_FILE ?= target/$(WASM_TARGET)/release/$(CONTRACT_PKG).wasm
CONTRACT_ID_FILE ?= .contract_id

.PHONY: all help build wasm deploy fund invoke test fmt lint clean

all: build

help:
	@echo "Makefile targets:"
	@echo "  build           Build the entire workspace"
	@echo "  wasm            Build contract WASM for $(CONTRACT_PKG)"
	@echo "  deploy          Deploy $(WASM_FILE) to $(NETWORK) via soroban"
	@echo "  fund ADDR=...   Fund a testnet address using Friendbot"
	@echo "  invoke FUNC=... Invoke a method on deployed contract (set CONTRACT_ID or CONTRACT_ID_FILE)"
	@echo "  test            Run cargo test"
	@echo "  fmt             Run cargo fmt"
	@echo "  lint            Run cargo clippy"
	@echo "  clean           Run cargo clean"

build:
	cargo build --workspace

wasm:
	@which cargo >/dev/null 2>&1 || (echo "cargo not found"; exit 1)
	@rustup target add $(WASM_TARGET) >/dev/null 2>&1 || true
	cargo build -p $(CONTRACT_PKG) --target $(WASM_TARGET) $(RELEASE_FLAG)

deploy: wasm
	@command -v soroban >/dev/null 2>&1 || (echo "soroban CLI not found; install via 'cargo install soroban-cli'"; exit 1)
	@echo "Deploying $(WASM_FILE) to network=$(NETWORK)"
	@soroban contract deploy --wasm $(WASM_FILE) --network $(NETWORK) | tee $(CONTRACT_ID_FILE)
	@echo "Contract ID stored in $(CONTRACT_ID_FILE)"

fund:
	@if [ -z "$(ADDR)" ]; then echo "Usage: make fund ADDR=G..."; exit 1; fi
	@if [ "$(NETWORK)" != "testnet" ]; then echo "Friendbot only available on testnet/futurenet"; exit 1; fi
	@echo "Funding $(ADDR) via Friendbot"
	@curl -sS "https://friendbot.stellar.org/?addr=$(ADDR)" || true

invoke:
	@command -v soroban >/dev/null 2>&1 || (echo "soroban CLI not found; install via 'cargo install soroban-cli'"; exit 1)
	@if [ -z "$(FUNC)" ]; then echo "Usage: make invoke FUNC=<method> [CONTRACT_ID=<id>] [ARGS='arg1 arg2']"; exit 1; fi
	@CONTRACT_ID=$${CONTRACT_ID:-$$(cat $(CONTRACT_ID_FILE) 2>/dev/null || true)}; \
	if [ -z "$$CONTRACT_ID" ]; then echo "Contract ID not set and $(CONTRACT_ID_FILE) missing"; exit 1; fi; \
	ARGS=$${ARGS:-}; \
	set -x; soroban contract invoke --id "$$CONTRACT_ID" --network $(NETWORK) --fn $(FUNC) --args $$ARGS

test:
	cargo test --workspace

fmt:
	cargo fmt --all

lint:
	cargo clippy --all-targets --all-features -- -D warnings

clean:
	cargo clean

