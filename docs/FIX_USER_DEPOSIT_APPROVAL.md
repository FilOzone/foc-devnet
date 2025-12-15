# Fix: ERC-20 Approval Required Before Deposit

## Problem

The `depositWithPermitAndApproveOperator` transaction was failing with status 0 because USER_0 had not approved the FilecoinPay contract to spend their USDFC tokens.

### Error Message
```
depositWithPermitAndApproveOperator transaction failed (status 0). 
Check transaction logs for details.
```

## Root Cause

The `depositWithPermitAndApproveOperator` function attempts to transfer USDFC tokens from USER_0's account to the FilecoinPay contract. However, this requires a prior **ERC-20 approval** to allow the contract to spend tokens on behalf of the user.

This is a standard ERC-20 pattern:
1. User calls `approve(spender, amount)` on the token contract
2. Contract can then call `transferFrom(user, destination, amount)`

## Solution

Added a two-step process in the `execute()` phase:

### Step 1: Approve USDFC Spending
Call `approve(address,uint256)` on the USDFC token contract:
- **Spender**: FilecoinPay contract address
- **Amount**: Deposit amount in wei (1,000 USDFC)

### Step 2: Verify Approval
Query `allowance(address,address)` on the USDFC contract to confirm:
- The allowance is set correctly
- FilecoinPay can spend at least the deposit amount

### Step 3: Deposit and Permit
Call `depositWithPermitAndApproveOperator` on FilecoinPay (original step)

## Code Changes

### New Function: `approve_usdfc_for_filecoin_pay()`

Location: `src/commands/start/user_deposit_permit/operations.rs`

```rust
pub fn approve_usdfc_for_filecoin_pay(
    usdfc_address: &str,
    filecoin_pay_address: &str,
    user_private_key: &str,
    amount_wei: &str,
) -> Result<(), Box<dyn Error>>
```

Calls:
```solidity
cast send <USDFC_ADDRESS> \
  "approve(address,uint256)" \
  <FILECOIN_PAY_ADDRESS> \
  <AMOUNT_WEI> \
  --private-key <USER_PRIVATE_KEY>
```

### New Function: `query_usdfc_allowance()`

Location: `src/commands/start/user_deposit_permit/operations.rs`

```rust
pub fn query_usdfc_allowance(
    usdfc_address: &str,
    user_eth_address: &str,
    filecoin_pay_address: &str,
) -> Result<String, Box<dyn Error>>
```

Calls:
```solidity
cast call <USDFC_ADDRESS> \
  "allowance(address,address)" \
  <USER_ADDRESS> \
  <FILECOIN_PAY_ADDRESS>
```

### Updated Execution Flow

Location: `src/commands/start/user_deposit_permit/user_deposit_permit_step.rs`

```rust
fn execute(&self, context: &mut StepContext) -> Result<(), Box<dyn Error>> {
    // ... get addresses and keys ...
    
    // Step 1: Approve FilecoinPay to spend USDFC
    approve_usdfc_for_filecoin_pay(...)?;
    
    // Verify approval
    let allowance = query_usdfc_allowance(...)?;
    // Check allowance >= deposit_amount
    
    // Step 2: Deposit and approve operator
    deposit_with_permit_and_approve_operator(...)?;
    
    // ...
}
```

## Expected Output

After the fix, the step should output:

```
Executing USER_0 Deposit and Operator Approval
  Approving FilecoinPay to spend USDFC tokens...
    Amount: 1000 USDFC tokens
  ✓ Approval successful
      Waiting 8 seconds for transaction confirmation...
  ✓ USDFC allowance verified: 1000000000000000000000 wei
  Calling depositWithPermitAndApproveOperator on FilecoinPay...
    Depositing 1000 USDFC tokens
    Approving WarmStorage as operator
    Lockup allowance: 2592000 seconds (30 days)
  ✓ Transaction successful
      Waiting 8 seconds for transaction confirmation...
  ✓ USER_0 deposit and operator approval complete
```

## Verification

The post-execution checks will verify:
1. USER_0's FilecoinPay balance equals the deposit amount
2. WarmStorage has proper operator allowances (rate, lockup, max)

## Why This Pattern?

This is the standard ERC-20 "approve then transfer" pattern used throughout DeFi:

1. **Security**: Users explicitly approve contracts to spend specific amounts
2. **Control**: Users can revoke or modify allowances at any time
3. **Transparency**: All approvals are visible on-chain

The `depositWithPermitAndApproveOperator` function likely uses `transferFrom()` internally, which requires this prior approval.

## Related Files

- `src/commands/start/user_deposit_permit/operations.rs` - Added approval functions
- `src/commands/start/user_deposit_permit/user_deposit_permit_step.rs` - Updated execution flow
- `docs/USER_DEPOSIT_PERMIT_STEP.md` - Documentation (should be updated)

## Testing

To test manually:

```bash
# Clean and restart
cargo run -- stop
cargo run -- start --regenesis --reset

# The USER_0 Deposit step should now succeed with approval
```

The step will now:
1. ✅ Approve FilecoinPay to spend USDFC
2. ✅ Verify allowance is set
3. ✅ Call depositWithPermitAndApproveOperator
4. ✅ Verify final balances and operator allowances
