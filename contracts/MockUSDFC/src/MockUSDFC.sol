// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import "@openzeppelin/contracts/token/ERC20/ERC20.sol";

/**
 * @title MockUSDFC
 * @dev Mock USDC token for testing FOC warm storage services
 */
contract MockUSDFC is ERC20 {
    uint8 private _decimals;

    /**
     * @dev Constructor that gives msg.sender all of initial supply.
     * @param initialSupply The initial supply of tokens (in wei, accounting for decimals)
     */
    constructor(uint256 initialSupply) ERC20("Mock USDC", "USDFC") {
        _decimals = 18;
        _mint(msg.sender, initialSupply);
    }

    /**
     * @dev Returns the number of decimals used for token amounts.
     */
    function decimals() public view virtual override returns (uint8) {
        return _decimals;
    }

    /**
     * @dev Mint new tokens (for testing purposes)
     * @param to Address to receive the tokens
     * @param amount Amount of tokens to mint
     */
    function mint(address to, uint256 amount) external {
        _mint(to, amount);
    }

    /**
     * @dev Burn tokens (for testing purposes)
     * @param from Address to burn tokens from
     * @param amount Amount of tokens to burn
     */
    function burn(address from, uint256 amount) external {
        _burn(from, amount);
    }
}
