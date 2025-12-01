// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import "forge-std/Script.sol";
import "../src/MockUSDFC.sol";

/**
 * @title VerifyMockUSDFC
 * @dev Script to verify MockUSDFC token contract functions
 * 
 * Usage:
 *   forge script script/Verify.s.sol:VerifyMockUSDFC \
 *     --rpc-url http://localhost:1234/rpc/v1 \
 *     --private-key <PRIVATE_KEY> \
 *     --sig "run(address)" <CONTRACT_ADDRESS>
 */
contract VerifyMockUSDFC is Script {
    function run(address tokenAddress) external {
        require(tokenAddress != address(0), "Invalid token address");

        MockUSDFC token = MockUSDFC(tokenAddress);

        console.log("=== Verifying MockUSDFC at", tokenAddress, "===");
        console.log("");

        // Test 1: Read basic token information
        console.log("1. Reading token metadata:");
        string memory name = token.name();
        string memory symbol = token.symbol();
        uint8 decimals = token.decimals();
        uint256 totalSupply = token.totalSupply();

        console.log("   Name:", name);
        console.log("   Symbol:", symbol);
        console.log("   Decimals:", decimals);
        console.log("   Total Supply:", totalSupply);
        
        require(keccak256(bytes(name)) == keccak256(bytes("Mock USDC")), "Invalid name");
        require(keccak256(bytes(symbol)) == keccak256(bytes("USDFC")), "Invalid symbol");
        require(decimals == 18, "Invalid decimals");
        console.log("   [OK] Metadata verification passed");
        console.log("");

        // Test 2: Check deployer balance
        console.log("2. Checking deployer balance:");
        address deployer = msg.sender;
        uint256 deployerBalance = token.balanceOf(deployer);
        console.log("   Deployer:", deployer);
        console.log("   Balance:", deployerBalance);
        console.log("   [OK] Balance check passed");
        console.log("");

        // Test 3: Test transfer (if we have balance)
        if (deployerBalance > 0) {
            console.log("3. Testing transfer:");
            address recipient = address(0x1234567890123456789012345678901234567890);
            uint256 transferAmount = 1000 * 10**18; // 1000 tokens

            if (deployerBalance >= transferAmount) {
                vm.startBroadcast();
                token.transfer(recipient, transferAmount);
                vm.stopBroadcast();

                uint256 recipientBalance = token.balanceOf(recipient);
                console.log("   Transferred", transferAmount, "to", recipient);
                console.log("   Recipient balance:", recipientBalance);
                require(recipientBalance == transferAmount, "Transfer failed");
                console.log("   [OK] Transfer test passed");
            } else {
                console.log("   [WARN] Insufficient balance for transfer test");
            }
        } else {
            console.log("3. Skipping transfer test (no balance)");
        }
        console.log("");

        // Test 4: Test mint function
        console.log("4. Testing mint function:");
        address mintRecipient = address(0x9876543210987654321098765432109876543210);
        uint256 mintAmount = 500 * 10**18; // 500 tokens

        uint256 totalSupplyBefore = token.totalSupply();
        
        vm.startBroadcast();
        token.mint(mintRecipient, mintAmount);
        vm.stopBroadcast();

        uint256 mintRecipientBalance = token.balanceOf(mintRecipient);
        uint256 totalSupplyAfter = token.totalSupply();

        console.log("   Minted", mintAmount, "to", mintRecipient);
        console.log("   Recipient balance:", mintRecipientBalance);
        console.log("   Total supply before:", totalSupplyBefore);
        console.log("   Total supply after:", totalSupplyAfter);
        
        require(mintRecipientBalance == mintAmount, "Mint failed");
        require(totalSupplyAfter == totalSupplyBefore + mintAmount, "Total supply not updated");
        console.log("   [OK] Mint test passed");
        console.log("");

        // Test 5: Test burn function
        console.log("5. Testing burn function:");
        uint256 burnAmount = 100 * 10**18; // 100 tokens

        totalSupplyBefore = token.totalSupply();
        uint256 mintRecipientBalanceBefore = token.balanceOf(mintRecipient);

        vm.startBroadcast();
        token.burn(mintRecipient, burnAmount);
        vm.stopBroadcast();

        uint256 mintRecipientBalanceAfter = token.balanceOf(mintRecipient);
        totalSupplyAfter = token.totalSupply();

        console.log("   Burned", burnAmount, "from", mintRecipient);
        console.log("   Balance before:", mintRecipientBalanceBefore);
        console.log("   Balance after:", mintRecipientBalanceAfter);
        console.log("   Total supply before:", totalSupplyBefore);
        console.log("   Total supply after:", totalSupplyAfter);
        
        require(mintRecipientBalanceAfter == mintRecipientBalanceBefore - burnAmount, "Burn failed");
        require(totalSupplyAfter == totalSupplyBefore - burnAmount, "Total supply not updated");
        console.log("   [OK] Burn test passed");
        console.log("");

        console.log("=== All verifications passed! ===");
    }
}
