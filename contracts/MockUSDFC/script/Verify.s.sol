// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

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
contract VerifyMockUSDFC {
    function run(address tokenAddress) external view {
        require(tokenAddress != address(0), "Invalid token address");

        MockUSDFC token = MockUSDFC(tokenAddress);

        string memory name = token.name();
        string memory symbol = token.symbol();
        uint8 decimals = token.decimals();

        require(keccak256(bytes(name)) == keccak256(bytes("Mock USDC")), "Invalid name");
        require(keccak256(bytes(symbol)) == keccak256(bytes("USDFC")), "Invalid symbol");
        require(decimals == 18, "Invalid decimals");
    }
}
