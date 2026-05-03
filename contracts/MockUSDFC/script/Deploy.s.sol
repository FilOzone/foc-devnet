// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import "../src/MockUSDFC.sol";

/**
 * @title DeployMockUSDFC
 * @dev Script to deploy MockUSDFC token contract
 * 
 * Usage:
 *   forge script script/Deploy.s.sol:DeployMockUSDFC \
 *     --rpc-url http://localhost:1234/rpc/v1 \
 *     --private-key <PRIVATE_KEY> \
 *     --broadcast
 */
contract DeployMockUSDFC {
    // Default initial supply: 1,000,000 tokens with 18 decimals
    uint256 constant DEFAULT_INITIAL_SUPPLY = 1_000_000 * 10**18;

    function run() external returns (MockUSDFC) {
        return new MockUSDFC(DEFAULT_INITIAL_SUPPLY);
    }
}
