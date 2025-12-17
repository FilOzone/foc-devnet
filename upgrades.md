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
**Status**: ⏳ Not Started

### Tasks:
- [ ] Create `src/commands/start/curio/mod.rs` (orchestrator)
- [ ] Create `src/commands/start/curio/pre_execute.rs` (verification)
- [ ] Create `src/commands/start/curio/execute.rs` (main setup)
- [ ] Create `src/commands/start/curio/post_execute.rs` (verification)
- [ ] Create `src/commands/start/curio/db_setup.rs` (DB migration & config)
- [ ] Create `src/commands/start/curio/daemon.rs` (daemon management)
- [ ] Create `src/commands/start/curio/storage.rs` (storage attach)
- [ ] Create `src/commands/start/curio/pdp.rs` (PDP key import)
- [ ] Create `src/commands/start/curio/verification.rs` (upload/download tests)

**Git Commit**: `refactor: curio step multi-file module structure`

---

## Milestone 4: Curio Execute - Base Layer Setup
**Status**: ⏳ Not Started

### Tasks:
- [ ] Implement `curio config new-cluster` for each SP
- [ ] Implement `curio config create` for pdp-layer config
- [ ] Handle volume mappings for curio-X instances
- [ ] Error handling for DB setup failures

**Git Commit**: `feat: curio base and pdp layer configuration`

---

## Milestone 5: Curio Execute - Daemon & Storage
**Status**: ⏳ Not Started

### Tasks:
- [ ] Start curio daemon with proper layers
- [ ] Implement storage attach for fast-storage
- [ ] Implement storage attach for long-term-storage
- [ ] Volume mount configuration for Docker
- [ ] Daemon health verification

**Git Commit**: `feat: curio daemon and storage management`

---

## Milestone 6: Curio Execute - PDP Key Import
**Status**: ⏳ Not Started

### Tasks:
- [ ] Implement PDP key import via JSON-RPC
- [ ] Verify returned address matches addresses.json
- [ ] Error handling for key import failures
- [ ] Add retry logic for API calls

**Git Commit**: `feat: curio pdp key import and verification`

---

## Milestone 7: Curio Post-Execute - Verification
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
