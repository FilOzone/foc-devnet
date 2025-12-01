# MockUSDFC Foundry Project Makefile
# 
# This Makefile provides convenient commands for deploying and verifying
# the MockUSDFC token contract.

.PHONY: help install build test clean deploy verify deploy-verify

# Default target
help:
	@echo "MockUSDFC Foundry Project"
	@echo ""
	@echo "Available targets:"
	@echo "  make install         - Install OpenZeppelin dependencies"
	@echo "  make build          - Build the contracts"
	@echo "  make test           - Run tests"
	@echo "  make clean          - Clean build artifacts"
	@echo "  make deploy         - Deploy MockUSDFC (requires PRIVATE_KEY)"
	@echo "  make verify         - Verify deployed contract (requires PRIVATE_KEY and CONTRACT_ADDRESS)"
	@echo "  make deploy-verify  - Deploy and verify in one step (requires PRIVATE_KEY)"
	@echo ""
	@echo "Environment variables:"
	@echo "  PRIVATE_KEY         - Private key for deployment (required)"
	@echo "  CONTRACT_ADDRESS    - Address of deployed contract (for verify)"
	@echo "  RPC_URL             - RPC endpoint (default: http://localhost:1234/rpc/v1)"
	@echo "  INITIAL_SUPPLY      - Initial token supply (default: 1000000000000000000000000)"
	@echo ""
	@echo "Example usage:"
	@echo "  make deploy PRIVATE_KEY=0xabc123..."
	@echo "  make verify PRIVATE_KEY=0xabc123... CONTRACT_ADDRESS=0xdef456..."
	@echo "  make deploy-verify PRIVATE_KEY=0xabc123..."

# Install OpenZeppelin dependencies
install:
	@echo "Installing dependencies..."
	forge install OpenZeppelin/openzeppelin-contracts@v5.0.0 --no-commit

# Build contracts
build:
	@echo "Building contracts..."
	forge build

# Run tests
test:
	@echo "Running tests..."
	forge test -vvv

# Clean build artifacts
clean:
	@echo "Cleaning build artifacts..."
	forge clean

# Deploy MockUSDFC
deploy:
	@if [ -z "$(PRIVATE_KEY)" ]; then \
		echo "Error: PRIVATE_KEY is required"; \
		echo "Usage: make deploy PRIVATE_KEY=0x..."; \
		exit 1; \
	fi
	@echo "Deploying MockUSDFC..."
	@RPC_URL=$${RPC_URL:-http://localhost:1234/rpc/v1}; \
	INITIAL_SUPPLY=$${INITIAL_SUPPLY:-1000000000000000000000000}; \
	forge script script/Deploy.s.sol:DeployMockUSDFC \
		--rpc-url $$RPC_URL \
		--private-key $(PRIVATE_KEY) \
		--broadcast \
		-vvv

# Verify deployed contract
verify:
	@if [ -z "$(PRIVATE_KEY)" ]; then \
		echo "Error: PRIVATE_KEY is required"; \
		echo "Usage: make verify PRIVATE_KEY=0x... CONTRACT_ADDRESS=0x..."; \
		exit 1; \
	fi
	@if [ -z "$(CONTRACT_ADDRESS)" ]; then \
		echo "Error: CONTRACT_ADDRESS is required"; \
		echo "Usage: make verify PRIVATE_KEY=0x... CONTRACT_ADDRESS=0x..."; \
		exit 1; \
	fi
	@echo "Verifying MockUSDFC at $(CONTRACT_ADDRESS)..."
	@RPC_URL=$${RPC_URL:-http://localhost:1234/rpc/v1}; \
	forge script script/Verify.s.sol:VerifyMockUSDFC \
		--rpc-url $$RPC_URL \
		--private-key $(PRIVATE_KEY) \
		--sig "run(address)" $(CONTRACT_ADDRESS) \
		-vvv

# Deploy and verify in one step
deploy-verify:
	@if [ -z "$(PRIVATE_KEY)" ]; then \
		echo "Error: PRIVATE_KEY is required"; \
		echo "Usage: make deploy-verify PRIVATE_KEY=0x..."; \
		exit 1; \
	fi
	@echo "Deploying and verifying MockUSDFC..."
	@RPC_URL=$${RPC_URL:-http://localhost:1234/rpc/v1}; \
	INITIAL_SUPPLY=$${INITIAL_SUPPLY:-1000000000000000000000000}; \
	echo "Step 1: Deploying contract..."; \
	OUTPUT=$$(forge script script/Deploy.s.sol:DeployMockUSDFC \
		--rpc-url $$RPC_URL \
		--private-key $(PRIVATE_KEY) \
		--broadcast \
		-vvv 2>&1); \
	echo "$$OUTPUT"; \
	CONTRACT_ADDRESS=$$(echo "$$OUTPUT" | grep "MockUSDFC deployed at:" | awk '{print $$NF}'); \
	if [ -z "$$CONTRACT_ADDRESS" ]; then \
		echo "Error: Failed to extract contract address from deployment output"; \
		exit 1; \
	fi; \
	echo ""; \
	echo "Contract deployed at: $$CONTRACT_ADDRESS"; \
	echo ""; \
	echo "Step 2: Waiting 5 seconds for transaction confirmation..."; \
	sleep 5; \
	echo ""; \
	echo "Step 3: Verifying contract..."; \
	forge script script/Verify.s.sol:VerifyMockUSDFC \
		--rpc-url $$RPC_URL \
		--private-key $(PRIVATE_KEY) \
		--sig "run(address)" $$CONTRACT_ADDRESS \
		-vvv
