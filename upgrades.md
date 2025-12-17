# Curio Multi-SP Implementation Upgrades

## Overview
Refactoring Curio setup to support multiple PDP Service Providers with proper verification.

---

## Milestone 1: Foundation - Multi-SP Infrastructure
**Status**: ✅ Complete

### Tasks:
- [x] Add `MAX_PDP_SP_COUNT` constant (value: 5)
- [x] Update genesis/key generation to create PDP_SP_1 through PDP_SP_5 (base-1)
- [x] Create pre-sealed sectors for N PDP service providers
- [x] Update reset command to delete curio volumes
- [x] Add config.toml support for number of active PDP SPs (via ACTIVE_PDP_SP_COUNT constant)

**Git Commit**: `f6b9604` - `feat: foundation for multi-sp curio infrastructure`

---

## Milestone 2: Yugabyte Multi-Instance Support
**Status**: ⏳ Not Started

### Tasks:
- [ ] Spawn multiple `foc-yugabyte-X` containers (base-1)
- [ ] Separate yugabyte networks for each instance
- [ ] Update volume mappings for multiple yugabyte instances
- [ ] Update docker utilities to handle yugabyte-1, yugabyte-2, etc.

**Git Commit**: `feat: yugabyte multi-instance support`

---

## Milestone 3: Curio Step Refactoring - Module Structure
**Status**: ✅ Complete

### Tasks:
- [x] Create `src/commands/start/curio/mod.rs` (orchestrator)
- [x] Create `src/commands/start/curio/pre_execute.rs` (verification)
- [x] Create `src/commands/start/curio/execute.rs` (main setup)
- [x] Create `src/commands/start/curio/post_execute.rs` (verification)
- [x] Create `src/commands/start/curio/db_setup.rs` (DB migration & config - stub)
- [x] Create `src/commands/start/curio/daemon.rs` (daemon management - stub)
- [x] Create `src/commands/start/curio/storage.rs` (storage attach - stub)
- [x] Create `src/commands/start/curio/pdp.rs` (PDP key import - stub)
- [x] Create `src/commands/start/curio/verification.rs` (upload/download tests - stub)

**Git Commit**: `f97823b` - `refactor: curio step multi-file module structure`

---

## Milestone 4: Curio Execute - Base Layer Setup
**Status**: ✅ Complete

### Tasks:
- [x] Implement `curio config new-cluster` for each SP
- [x] Implement `curio config create` for pdp-layer config
- [x] Handle volume mappings for curio-X instances
- [x] Error handling for DB setup failures
- [x] Implement PDP key import via JSON-RPC
- [x] Verify returned address matches addresses.json

**Git Commit**: `1502211` - `feat: implement curio database setup and key management`

---

## Milestone 5: Curio Execute - Daemon & Container Orchestration
**Status**: ⏳ Not Started

### Tasks:
- [ ] Implement Docker container creation for each Curio SP
- [ ] Configure volume mounts (curio data, fast-storage, long-term-storage, etc.)
- [ ] Configure environment variables (DB connection, Lotus API, contracts)
- [ ] Start curio daemon with proper layers (`curio run --nosync --layers seal,post,pdp-only,gui`)
- [ ] Wait for daemon readiness (API endpoint check at http://localhost:4701)
- [ ] Daemon health verification

**Technical Notes:**
- Container name: `foc-curio-{sp_index}-{run_id}`
- Volume mounts: `.curio`, `fast-storage`, `long-term-storage`, `lotus-data`, `genesis-sectors/pdp-sp-{sp_index}`
- Environment: DB connection to `foc-yugabyte-{sp_index}`, Lotus API, contract addresses
- Networking: Connect to both yugabyte and filecoin networks

**Git Commit**: Not yet completed

---

## Milestone 6: Curio Post-Execute - Verification
**Status**: ⏳ Not Started

### Tasks:
- [ ] Implement PDP ping endpoint check
- [ ] Add reqwest dependency for HTTP checks
- [ ] Generate random test file (1KB)
- [ ] Implement pdptool upload-piece execution
- [ ] Parse piece CID from upload output
- [ ] Implement piece download via HTTP
- [ ] Verify downloaded data matches uploaded data
- [ ] Comprehensive error reporting

**Git Commit**: `feat: curio post-execute verification with upload/download tests`

---

## Milestone 8: Integration & Testing
**Status**: ⏳ Not Started

### Tasks:
- [ ] Integration testing with single SP
- [ ] Integration testing with multiple SPs
- [ ] Update documentation
- [ ] Performance testing
- [ ] Clean up old curio implementation

**Git Commit**: `feat: complete curio multi-sp implementation`

---

## Notes & Issues

### Design Decisions:
- Base-1 numbering for PDP_SP_X (PDP_SP_1 through PDP_SP_5)
- Each Curio SP gets isolated yugabyte instance
- Maximum 5 PDP SPs (configurable via MAX_PDP_SP_COUNT)

### Technical Considerations:
- Storage paths: `/home/foc-user/curio/fast-storage` and `/home/foc-user/curio/long-term-storage`
- Curio API port: 4701 (web RPC)
- PDP subsystem port: 4702
- Storage attach machine: 127.0.0.1:12300

### Dependencies to Add:
- `reqwest` for HTTP verification in post-execute

---

Last Updated: 2025-12-17

## Summary of Progress

### ✅ Completed Milestones:
1. **Foundation** - Multi-SP infrastructure constants and path functions
2. **Module Structure** - Refactored Curio step into clean, modular architecture

### 🔄 Current Status:
Working on Milestone 4: Implementing actual database setup logic for Curio base and PDP layers.

### 📋 Next Steps:
1. Implement database setup (curio config new-cluster, curio config create)
2. Implement daemon startup with proper container orchestration
3. Implement storage attachment logic
4. Implement PDP key import via JSON-RPC
5. Implement verification with actual HTTP requests and pdptool integration
6. Add Yugabyte multi-instance support

### 🎯 Key Architectural Decisions:
- **Base-1 Numbering**: PDP_SP_1 through PDP_SP_5 (not base-0)
- **Modular Design**: Each Curio module <150 lines, clear separation of concerns
- **Stub Implementation**: All functions compile and execute, but logic is placeholder
- **Sequential Miner IDs**: PDP SPs get t01002, t01003, etc.
- **Isolated Resources**: Each Curio SP has own database, storage, and pre-sealed sectors

### 🔧 Technical Notes:
- `ACTIVE_PDP_SP_COUNT` constant controls how many SPs are actually initialized (default: 1)
- `MAX_PDP_SP_COUNT` constant sets maximum possible SPs (5)
- All curio volumes deleted on reset/regenesis
- Pre-execute verifies chain is progressing before starting Curio

---
