# StellarAid Contract Workspace Makefile
# Provides convenient commands for building, testing, and deploying Soroban contracts

.PHONY: build test clean help deploy-testnet deploy-mainnet fmt lint wasm

# Default target
.DEFAULT_GOAL := help

# Colors for output
BLUE := \033[36m
GREEN := \033[32m
YELLOW := \033[33m
RED := \033[31m
NC := \033[0m # No Color

help: ## Show this help message
	@echo "$(BLUE)StellarAid Contract Workspace$(NC)"
	@echo ""
	@echo "Available targets:"
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | sort | awk 'BEGIN {FS = ":.*?## "}; {printf "  $(GREEN)%-15s$(NC) %s\n", $$1, $$2}'

build: ## Build all workspace crates
	@echo "$(BLUE)Building all workspace crates...$(NC)"
	cargo build --workspace
	@echo "$(GREEN)Build complete!$(NC)"

wasm: ## Build WASM contracts for deployment
	@echo "$(BLUE)Building WASM contracts...$(NC)"
	cargo build -p donation --target wasm32-unknown-unknown --release
	cargo build -p withdrawal --target wasm32-unknown-unknown --release
	cargo build -p campaign --target wasm32-unknown-unknown --release
	@echo "$(GREEN)WASM build complete!$(NC)"
	@echo "$(YELLOW)WASM files located in target/wasm32-unknown-unknown/release/$(NC)"

test: ## Run all tests
	@echo "$(BLUE)Running all tests...$(NC)"
	cargo test --workspace
	@echo "$(GREEN)Tests complete!$(NC)"

test-contracts: ## Run contract tests only
	@echo "$(BLUE)Running contract tests...$(NC)"
	cargo test -p donation
	cargo test -p withdrawal
	cargo test -p campaign
	@echo "$(GREEN)Contract tests complete!$(NC)"

deploy-testnet: wasm ## Deploy contracts to Soroban testnet
	@echo "$(YELLOW)Deploying to testnet...$(NC)"
	@echo "$(BLUE)Deploying donation contract...$(NC)"
	soroban contract deploy --wasm target/wasm32-unknown-unknown/release/donation.wasm --source-account default --network testnet
	@echo "$(BLUE)Deploying withdrawal contract...$(NC)"
	soroban contract deploy --wasm target/wasm32-unknown-unknown/release/withdrawal.wasm --source-account default --network testnet
	@echo "$(BLUE)Deploying campaign contract...$(NC)"
	soroban contract deploy --wasm target/wasm32-unknown-unknown/release/campaign.wasm --source-account default --network testnet
	@echo "$(GREEN)Testnet deployment complete!$(NC)"

deploy-mainnet: wasm ## Deploy contracts to Soroban mainnet
	@echo "$(YELLOW)Deploying to mainnet...$(NC)"
	@echo "$(RED)WARNING: This will use real funds!$(NC)"
	@read -p "Are you sure? [y/N] " confirm && [ $$confirm = y ] || exit 1
	@echo "$(BLUE)Deploying donation contract...$(NC)"
	soroban contract deploy --wasm target/wasm32-unknown-unknown/release/donation.wasm --source-account default --network mainnet
	@echo "$(BLUE)Deploying withdrawal contract...$(NC)"
	soroban contract deploy --wasm target/wasm32-unknown-unknown/release/withdrawal.wasm --source-account default --network mainnet
	@echo "$(BLUE)Deploying campaign contract...$(NC)"
	soroban contract deploy --wasm target/wasm32-unknown-unknown/release/campaign.wasm --source-account default --network mainnet
	@echo "$(GREEN)Mainnet deployment complete!$(NC)"

fmt: ## Format all code
	@echo "$(BLUE)Formatting code...$(NC)"
	cargo fmt --all
	@echo "$(GREEN)Formatting complete!$(NC)"

lint: ## Run clippy linter
	@echo "$(BLUE)Running linter...$(NC)"
	cargo clippy --workspace -- -D warnings
	@echo "$(GREEN)Linting complete!$(NC)"

clean: ## Clean build artifacts
	@echo "$(BLUE)Cleaning build artifacts...$(NC)"
	cargo clean
	@echo "$(GREEN)Clean complete!$(NC)"

install-cli: ## Install Soroban CLI
	@echo "$(BLUE)Installing Soroban CLI...$(NC)"
	cargo install --locked soroban-cli
	@echo "$(GREEN)Soroban CLI installed!$(NC)"

# Development utilities
dev-setup: install-cli ## Setup development environment
	@echo "$(BLUE)Setting up development environment...$(NC)"
	@rustup target add wasm32-unknown-unknown
	@echo "$(GREEN)Development environment ready!$(NC)"
