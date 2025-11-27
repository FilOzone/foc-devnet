# Refactor Tasks for foc-localnet

## Overview
This document tracks the refactoring of the foc-localnet codebase according to the following rules:
- No magic numbers (e.g., 20, 150); replace with named constants
- Split files longer than 350 lines into smaller modules
- Split functions longer than 20 lines into smaller functions/sub-functions
- Add descriptive docstrings to all functions

## Current Status
- **Started:** 27 November 2025
- **Completed Tasks:** 0
- **Pending Tasks:** TBD after analysis

## Task Breakdown

### Phase 1: Analysis
1. Identify all Rust source files and their line counts ✅
2. Analyze each file for:
   - Functions longer than 20 lines
   - Magic numbers
   - Missing docstrings
3. Identify files longer than 350 lines for splitting ✅

### Files longer than 350 lines:
- src/commands/start/foc_deploy.rs: 820 lines
- src/commands/start/lotus_miner.rs: 529 lines
- src/commands/start/curio.rs: 371 lines
- src/commands/start/lotus.rs: 527 lines
- src/commands/init/docker.rs: 515 lines
- src/commands/status/docker.rs: 456 lines
- src/commands/build/mod.rs: 332 lines

### Phase 2: Refactoring
- [ ] Replace magic numbers with constants
- [ ] Split long functions
- [ ] Split long files into modules
- [ ] Add docstrings to functions

### Phase 3: Validation
- [ ] Ensure code compiles after each change
- [ ] Run tests if available
- [ ] Verify no regressions

## Progress Log
- 27 Nov 2025: Created this management file and initial plan