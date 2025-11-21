# Key Clarifying Questions for Localnet Implementation

Before finalizing the implementation, please review these questions:

## 1. Genesis Block Creation Process

**Current Implementation:**
- BLS keys are generated and stored in separate volumes
- Pre-sealing creates sectors
- Lotus daemon creates genesis automatically with `--lotus-make-genesis`

**Documentation Approach:**
```bash
./lotus-seed genesis new localnet.json
./lotus-seed genesis set-signers --threshold=2 --signers <key-1> --signers <key-2> localnet.json
./lotus-seed genesis add-miner localnet.json ~/.genesis-sectors/pre-seal-t01000.json
```

**Questions:**
- Should we follow the documentation approach exactly?
- How do we extract the BLS addresses from the keyinfo files to use in `set-signers`?
- The current BLS keys are stored in separate directories - is this the right structure?

## 2. Wallet Import for Genesis Miner

The documentation shows:
```bash
./lotus wallet import --as-default ~/.genesis-sectors/pre-seal-t01000.key
```

**Questions:**
- Where is `pre-seal-t01000.key` created during pre-sealing?
- Should this import happen automatically in our startup sequence?
- Should it be imported into Lotus, Lotus-Miner, or both containers?

## 3. Curio Configuration and Initialization

**Current Implementation:**
- Creates a default config with `curio config default`
- Starts Curio with connection to YugabyteDB on localhost:5433

**Questions:**
- Does Curio need explicit connection configuration to the Lotus daemon?
- Does it need database initialization (schema creation) in YugabyteDB?
- Are there Curio-specific settings for 2KiB sectors or local devnet mode?

## 4. Container Networking Strategy

**Current Mix:**
- Lotus: Port mappings (-p 1234:1234 -p 1235:1235)
- Lotus-Miner: Host network (--network host)
- YugabyteDB: Port mappings
- Curio: Host network (--network host)

**Questions:**
- Should we use a custom Docker bridge network instead for better isolation?
- Is host networking acceptable for local development?
- The requirement says "None of these containers access internet" - how strictly should this be enforced?

## 5. Build Targets for Lotus

**Current Build:**
```bash
make 2k
make lotus-shed
cp lotus lotus-miner lotus-shed lotus-seed
```

**Questions:**
- Does `make 2k` build `lotus-seed`? (We're copying it but not explicitly building it)
- Should we run `make lotus-seed` explicitly?
- Are there any other tools we should build?

## 6. Container Data Persistence

**Current Volumes:**
- `~/.foc-localnet/artifacts/docker/volumes/lotus-keys/` - BLS keys
- `~/.foc-localnet/artifacts/docker/volumes/genesis-sectors/` - Pre-sealed sectors
- `~/.foc-localnet/artifacts/docker/volumes/genesis/` - Genesis files
- `<volumes-dir>/lotus-data/` - Lotus runtime data
- `<volumes-dir>/lotus-miner-data/` - Miner runtime data
- `<volumes-dir>/curio-data/` - Curio runtime data

**Questions:**
- Is this structure appropriate?
- Should genesis-related volumes be in artifacts (persistent) while runtime data is in temp volumes?
- Can users safely delete `<volumes-dir>` and restart with existing genesis?

## 7. Error Handling and Rollback

**Current Implementation:**
- Step-by-step execution with pre/execute/post phases
- Framework supports rollback but not explicitly implemented

**Questions:**
- If Lotus-Miner fails to start, should we automatically stop Lotus?
- Should there be a partial-cleanup mode vs full cleanup on error?
- What's the expected behavior for users when startup fails partway through?

## Recommended Priority

**High Priority (blocks testing):**
1. Genesis block creation process (#1)
2. Wallet import for genesis miner (#2)
3. Lotus build targets (#5)

**Medium Priority (affects functionality):**
4. Curio initialization (#3)
5. Container data persistence (#6)

**Low Priority (polish):**
6. Container networking strategy (#4)
7. Error handling and rollback (#7)

## How to Provide Answers

Please provide answers in the format:

```markdown
### Answer to Question X

**Decision:** [Your decision]

**Rationale:** [Why this approach]

**Implementation Notes:** [Any specific guidance for implementation]
```
