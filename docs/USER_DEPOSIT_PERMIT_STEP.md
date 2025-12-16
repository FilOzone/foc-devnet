# USER_0 Deposit and Permit Operator Step

## Overview

This implementation adds a new step to the foc-localnet startup sequence that sets up USER_0 for deal making by:

1. Depositing USDFC tokens into the FilecoinPay contract
2. Approving WarmStorage as an operator with rate and lockup allowance limits

This step executes after the PDP Service Provider Registration step and before Yugabyte/Curio initialization.

## Architecture

### Module Structure

```
src/commands/start/user_deposit_permit/
├── mod.rs                          # Module exports
├── constants.rs                    # Configuration constants
├── operations.rs                   # Contract interaction operations
└── user_deposit_permit_step.rs    # Step implementation
```

### Key Components

#### 1. Constants (`constants.rs`)

- **Deposit Amount**: 50,000 USDFC tokens (out of 100,000 funded)
- **Lockup Allowance**: 30 days (2,592,000 seconds) - required for FWSS
- **Rate Allowance**: Max uint256 (unlimited rate)
- **Max Allowance**: Max uint256 (unlimited operations)
- **Transaction Wait**: 8 seconds for confirmation

#### 2. Operations (`operations.rs`)

Key functions:

- `deposit_with_permit_and_approve_operator()` - Main contract interaction
  - Calls `depositWithPermitAndApproveOperator(address,uint256,address,uint256,uint256,uint256)`
  - Parameters: token address, amount, operator, rate allowance, lockup allowance, max allowance
  
- `query_filecoin_pay_balance()` - Verify deposit balance
  - Calls `balanceOf(address)` on FilecoinPay
  
- `query_operator_allowance()` - Verify operator approval
  - Calls `operatorAllowance(address,address)` on FilecoinPay
  - Returns tuple: (rateAllowance, lockupAllowance, maxAllowance)

- `token_amount_to_wei()` - Convert token amount to wei (18 decimals)

#### 3. Step Implementation (`user_deposit_permit_step.rs`)

Implements the `Step` trait with three phases:

**Pre-Execute**:
- Verify Lotus is running
- Load and verify contract addresses (FilecoinPay, WarmStorage, USDFC)
- Get USER_0 Ethereum address
- Basic USDFC balance check (assumes funding step completed)

**Execute**:
- Retrieve USER_0 private key
- Calculate deposit amount in wei (50,000 * 10^18)
- Call `depositWithPermitAndApproveOperator` with:
  - USDFC token address
  - 50,000 USDFC deposit amount
  - WarmStorage operator address
  - Max uint256 rate allowance
  - 30 days lockup allowance
  - Max uint256 max allowance
- Store success flag in context

**Post-Execute**:
- Query and verify FilecoinPay balance ≥ 50,000 USDFC
- Query operator allowance for WarmStorage
- Verify lockup allowance ≥ 30 days
- Display all allowances for verification

## Contract Addresses Used

The step requires these contract addresses from `contract_addresses.json`:

1. **FilecoinPay**: `foc_contracts.filecoin_pay_v1_contract`
2. **WarmStorage**: `foc_contracts.filecoin_warm_storage_service_proxy`
3. **USDFC**: `contracts.usdfc`

## Integration with Startup Sequence

### Position in Startup

```
1. Lotus
2. Lotus-Miner
3. ETH Account Funding
4. USDFC Deploy
5. USDFC Funding          ← USER_0 receives 100,000 USDFC
6. Multicall3 Deploy
7. FOC Deploy
8. PDP SP Registration
9. USER_0 Deposit & Permit ← NEW STEP (deposits 50,000 USDFC)
10. Yugabyte
11. Curio
```

### Dependencies

**Requires**:
- Lotus running (FEVM enabled)
- FOC contracts deployed (FilecoinPay, WarmStorage)
- USER_0 funded with USDFC (from USDFC Funding step)

**Provides**:
- USER_0 ready for deal making via Synapse SDK
- FilecoinPay deposit balance for USER_0
- WarmStorage operator approval with proper limits

## Usage

The step runs automatically as part of the startup sequence:

```bash
cargo run -- start --regenesis --reset
```

After successful completion:
- USER_0 has 50,000 USDFC deposited in FilecoinPay
- WarmStorage can create and modify payment rails for USER_0
- WarmStorage can lock funds up to 30 days
- No rate or operation limits on WarmStorage

## Verification

Post-execution checks verify:

1. **FilecoinPay Balance**: USER_0 has ≥ 50,000 USDFC deposited
2. **Rate Allowance**: Set to max uint256 (unlimited)
3. **Lockup Allowance**: ≥ 2,592,000 seconds (30 days)
4. **Max Allowance**: Set to max uint256 (unlimited)

## Error Handling

Possible failures:

- **Pre-execute**: Missing contract addresses, Lotus not running
- **Execute**: Transaction failure, insufficient gas, contract error
- **Post-execute**: Insufficient balance, incorrect allowances

All errors provide detailed context with contract addresses and expected values.

## Testing

### Manual Verification

After startup completes, you can verify the setup:

```bash
# Check FilecoinPay balance for USER_0
docker run --rm --network host foc-builder bash -c \
  "cast call <FILECOIN_PAY_ADDRESS> 'balanceOf(address)' <USER_0_ETH_ADDRESS> \
  --rpc-url http://localhost:1234/rpc/v1"

# Check operator allowance
docker run --rm --network host foc-builder bash -c \
  "cast call <FILECOIN_PAY_ADDRESS> 'operatorAllowance(address,address)' \
  <USER_0_ETH_ADDRESS> <WARM_STORAGE_ADDRESS> \
  --rpc-url http://localhost:1234/rpc/v1"
```

### Expected Output

The step should output:

```
Starting step: USER_0 Deposit and Operator Approval
  Running pre-execution checks...
  ✓ Lotus is running
  ✓ FilecoinPay: 0x...
  ✓ WarmStorage: 0x...
  ✓ USDFC: 0x...
  ✓ USER_0 ETH address: 0x...
  ✓ USER_0 USDFC balance check (assumed from funding step)

  Running execution...
  Calling depositWithPermitAndApproveOperator on FilecoinPay...
    Depositing 50000 USDFC tokens
    Approving WarmStorage as operator
    Lockup allowance: 2592000 seconds (30 days)
  ✓ Transaction successful
      Waiting 8 seconds for transaction confirmation...
  ✓ USER_0 deposit and operator approval complete

  Running post-execution checks...
  ✓ USER_0 FilecoinPay balance: 50000000000000000000000 wei (50000 USDFC)
  ✓ Operator allowances for WarmStorage:
      Rate allowance: 115792089237316195423570985008687907853269984665640564039457584007913129639935
      Lockup allowance: 2592000 seconds
      Max allowance: 115792089237316195423570985008687907853269984665640564039457584007913129639935
  ✓ All verifications passed

Step completed: USER_0 Deposit and Operator Approval (took 15.2s)
```

## Next Steps

After this step completes, USER_0 is ready to:

1. Create storage deals via Synapse SDK
2. Use WarmStorage for warm storage operations
3. Have payment rails automatically created by WarmStorage
4. Have funds locked for up to 30 days per deal

The setup follows the same pattern as the PDP Service Provider Registration step, maintaining consistency with the existing codebase architecture.
