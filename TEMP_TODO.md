# FOC Deploy Implementation TODO

## Context
We need to add support for deploying filecoin-onchain-contracts (FOC) on the Lotus node with FEVM enabled. This is required before Curio can start, as Curio depends on these contracts.

## Current Status
- ✅ Lotus node and Lotus-miner can start and build blocks/tipsets
- ✅ Yugabyte and Curio can start after Lotus is healthy
- ✅ Added filecoin-services repository configuration (v1.0.0 default)
- ✅ Added GLOBAL_FIL_FAUCET (NUM_PREFUNDED_KEYS = 1)
- ✅ FEVM enabled in Lotus with Ethereum RPC support
- ✅ FOCDeploy step created with account setup and fund transfers
- ✅ MockUSDFC token created for local testing (replaces production USDFC)
- ✅ Full contract deployment implemented with deploy-all-warm-storage.sh
- ✅ Contract addresses captured and saved to foc-contract-addresses.json
- 🔄 Ready for: Manual testing of full flow
- ❌ Pending: Update status command to show FOC deployment info

## Implementation Tasks

### Phase 1: Repository Management
- [x] Add `filecoin-services` repository to config structure
  - [x] Add to config.rs with default git URL and v1.0.0 tag
  - [x] Add repository selection during `init` command
  - [x] Add to paths.rs for repository path function
  - [N/A] Build step not needed (contains scripts, not compiled code)

### Phase 2: Account/Faucet Setup
- [x] Add `GLOBAL_FIL_FAUCET` pre-funded account in genesis
  - [x] Set NUM_PREFUNDED_KEYS = 1 in genesis/constants.rs
  - [x] Document as GLOBAL_FIL_FAUCET with 50,000 FIL tokens
- [ ] Add `FEVM_FAUCET` Ethereum address
  - [ ] Create f4/delegated address for Ethereum compatibility
  - [ ] Set up transfer from GLOBAL_FIL_FAUCET to FEVM_FAUCET
- [ ] Add `FOC_DEPLOYER` Ethereum address
  - [ ] Create/import address for contract deployment
  - [ ] Set up transfer from FEVM_FAUCET to FOC_DEPLOYER

### Phase 3: FEVM Enablement in Lotus
- [x] Update Lotus configuration to enable FEVM
  - [x] Add EnableEthRPC = true in [Fevm] section of config.toml
  - [x] Config file modified and daemon restarted after creation
  - [x] Reference: https://lotus.filecoin.io/lotus/developers/local-network/#fevm-features
- [x] Add post-execution tests for FEVM
  - [x] Test Ethereum RPC is available
  - [x] Test basic eth_* RPC calls work (eth_blockNumber)
  - [x] Add verification step after Lotus starts

### Phase 4: FOC Deploy Step
- [x] Create new start step: `FOCDeploy`
  - [x] Add to start/mod.rs step enum
  - [x] Create start/foc_deploy.rs module
  - [x] Insert step between Lotus-miner and Yugabyte
- [x] Implement account setup and fund transfers
  - [x] Import GLOBAL_FIL_FAUCET key into Lotus wallet
  - [x] Create FEVM_FAUCET f4 address (10,000 FIL)
  - [x] Create FOC_DEPLOYER f4 address (5,000 FIL)
  - [x] Transfer chain: GLOBAL → FEVM_FAUCET → FOC_DEPLOYER
  - [x] Export FOC_DEPLOYER private key for contract deployment
- [x] Implement deployment logic
  - [x] Set up foc-builder container with forge/cast/jq tools (Foundry installed)
  - [x] Deploy MockUSDFC token (toy ERC-20 for local testing)
  - [x] Execute deploy-all-warm-storage.sh script with proper env vars
  - [x] Pass USDFC_TOKEN_ADDRESS env var to deployment script
  - [x] Script location: filecoin-services:service_contracts/tools/deploy-all-warm-storage.sh
  - [x] Handle deployment outputs and contract addresses
- [x] Add verification/health checks
  - [x] Verify contracts are deployed
  - [x] Save contract addresses for Curio to use (foc-contract-addresses.json)
  - [x] Add status reporting for deployed contracts

### Phase 5: Integration & Testing
- [ ] Update build system for filecoin-services
  - [ ] Add Dockerfile if needed (likely in docker/)
  - [ ] Add volumes_map.toml for foc-builder
- [ ] Update start sequence
  - [ ] Lotus → Lotus-miner → FOCDeploy → Yugabyte → Curio
  - [ ] Add dependencies and health checks
- [ ] Update status command
  - [ ] Show filecoin-services version
  - [ ] Show FOC deployment status
  - [ ] Show deployed contract addresses
- [ ] Manual testing
  - [ ] Test full flow from init to Curio startup
  - [ ] Verify Curio can interact with deployed contracts

## Next Steps (Immediate)
1. ~~Examine current repository structure to understand patterns~~ ✅
2. ~~Add filecoin-services repository configuration~~ ✅
3. ~~Implement FEVM enablement in Lotus config~~ ✅
4. ~~Create FOCDeploy step with account setup~~ ✅
5. ~~Implement contract deployment execution~~ ✅
6. **Test full flow manually**
   - Run `cargo run init` and `cargo run build lotus`
   - Run `cargo run start` and verify all steps complete
   - Check foc-contract-addresses.json is created
   - Verify Lotus, Lotus-miner, and FOCDeploy all succeed
   - Document any issues encountered
7. **Update status command** to show FOC deployment info
   - Read foc-contract-addresses.json
   - Display deployed contract addresses
   - Show MockUSDFC token address
8. **Create usage documentation**
   - Document how Curio will use the deployed contracts
   - Explain the fund transfer chain
   - Provide troubleshooting guide

## Notes
- Script URL: https://raw.githubusercontent.com/FilOzone/filecoin-services/refs/heads/main/service_contracts/tools/deploy-all-warm-storage.sh
- Git repo: git@github.com:FilOzone/filecoin-services.git or https://github.com/FilOzone/filecoin-services.git
- Default tag: v1.0.0
- Conventional commits should be used for each milestone

## Questions/Decisions Needed
- [x] Does filecoin-services need to be built, or just cloned? → Just cloned (contains scripts)
- [x] What are the exact FEVM config parameters for Lotus? → EnableEthRPC = true in [Fevm] section
- [x] How to handle USDFC token for local testing? → Created MockUSDFC.sol toy ERC-20
- [ ] What are the exact environment variables needed for deploy script?
- [ ] How should contract addresses be stored/passed to Curio?
