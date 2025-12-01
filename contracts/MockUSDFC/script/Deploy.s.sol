// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import "forge-std/Script.sol";
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
contract DeployMockUSDFC is Script {
    // Default initial supply: 1,000,000 tokens with 18 decimals
    uint256 constant DEFAULT_INITIAL_SUPPLY = 1_000_000 * 10**18;

    function run() external returns (MockUSDFC) {
        // Get initial supply from environment variable or use default
        uint256 initialSupply = vm.envOr("INITIAL_SUPPLY", DEFAULT_INITIAL_SUPPLY);

        console.log("Deploying MockUSDFC with initial supply:", initialSupply);
        console.log("Deployer address:", msg.sender);

        // Start broadcast with a high gas limit for FEVM
        vm.startBroadcast();

        // Deploy contract
        MockUSDFC token = new MockUSDFC(initialSupply);

        vm.stopBroadcast();

        console.log("MockUSDFC deployed at:", address(token));
        console.log("Token name:", token.name());
        console.log("Token symbol:", token.symbol());
        console.log("Token decimals:", token.decimals());
        console.log("Total supply:", token.totalSupply());
        console.log("Deployer balance:", token.balanceOf(msg.sender));

        return token;
    }
}
