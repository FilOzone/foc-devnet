# FOC Deploy Implementation TODO

## Context
We need to add support for deploying filecoin-onchain-contracts (FOC) on the Lotus node with FEVM enabled. This is required before Curio can start, as Curio depends on these contracts.

## Current Status
- ✅ Lotus node and Lotus-miner can start and build blocks/tipsets
- ✅ Yugabyte and Curio can start after Lotus is healthy
- ✅ Added filecoin-services repository configuration (v1.0.0 default)
- ✅ Added GLOBAL_FIL_FAUCET (NUM_PREFUNDED_KEYS = 1)
- 🔄 Working on: FEVM enablement and FOC deployment step
- ❌ Missing: FOC contract deployment between Lotus and Curio startup

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
- [ ] Update Lotus configuration to enable FEVM
  - [ ] Add EnableEthRPC = true in [Fevm] section of config.toml
  - [ ] Config file needs to be created/modified in container after daemon starts
  - [ ] Reference: https://lotus.filecoin.io/lotus/developers/local-network/#fevm-features
- [ ] Add post-execution tests for FEVM
  - [ ] Test Ethereum RPC is available (port may be different or same as Lotus API)
  - [ ] Test basic eth_* RPC calls work
  - [ ] Add verification step after Lotus starts

### Phase 4: FOC Deploy Step
- [ ] Create new start step: `FOCDeploy`
  - [ ] Add to start/mod.rs step enum
  - [ ] Create start/foc_deploy.rs module
  - [ ] Insert step between Lotus-miner and Yugabyte
- [ ] Implement deployment logic
  - [ ] Use foc-builder container
  - [ ] Execute deploy-all-warm-storage.sh script
  - [ ] Script location: filecoin-services:service_contracts/tools/deploy-all-warm-storage.sh
  - [ ] Handle deployment outputs and contract addresses
- [ ] Add verification/health checks
  - [ ] Verify contracts are deployed
  - [ ] Save contract addresses for Curio to use
  - [ ] Add status reporting for deployed contracts

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
1. Examine current repository structure to understand patterns
2. Add filecoin-services repository configuration
3. Implement FEVM enablement in Lotus config
4. Test each phase incrementally

## Notes
- Script URL: https://raw.githubusercontent.com/FilOzone/filecoin-services/refs/heads/main/service_contracts/tools/deploy-all-warm-storage.sh
- Git repo: git@github.com:FilOzone/filecoin-services.git or https://github.com/FilOzone/filecoin-services.git
- Default tag: v1.0.0
- Conventional commits should be used for each milestone

## Questions/Decisions Needed
- [ ] Does filecoin-services need to be built, or just cloned?
- [ ] What are the exact environment variables needed for deploy script?
- [ ] How should contract addresses be stored/passed to Curio?
- [ ] What are the exact FEVM config parameters for Lotus?
