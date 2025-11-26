// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

/**
 * @title MockUSDFC
 * @dev Simple ERC-20 token for local testing
 * This is a toy implementation of USDFC for use in foc-localnet development.
 * 
 * Features:
 * - Standard ERC-20 interface
 * - Mintable by owner
 * - Initial supply minted to deployer
 * - No access control beyond owner minting
 */
contract MockUSDFC {
    string public constant name = "Mock USD Filecoin";
    string public constant symbol = "USDFC";
    uint8 public constant decimals = 18;
    
    uint256 public totalSupply;
    address public owner;
    
    mapping(address => uint256) public balanceOf;
    mapping(address => mapping(address => uint256)) public allowance;
    
    event Transfer(address indexed from, address indexed to, uint256 value);
    event Approval(address indexed owner, address indexed spender, uint256 value);
    event Mint(address indexed to, uint256 value);
    
    modifier onlyOwner() {
        require(msg.sender == owner, "Only owner can call this function");
        _;
    }
    
    constructor(uint256 initialSupply) {
        owner = msg.sender;
        _mint(msg.sender, initialSupply);
    }
    
    function transfer(address to, uint256 value) public returns (bool) {
        require(balanceOf[msg.sender] >= value, "Insufficient balance");
        balanceOf[msg.sender] -= value;
        balanceOf[to] += value;
        emit Transfer(msg.sender, to, value);
        return true;
    }
    
    function approve(address spender, uint256 value) public returns (bool) {
        allowance[msg.sender][spender] = value;
        emit Approval(msg.sender, spender, value);
        return true;
    }
    
    function transferFrom(address from, address to, uint256 value) public returns (bool) {
        require(balanceOf[from] >= value, "Insufficient balance");
        require(allowance[from][msg.sender] >= value, "Insufficient allowance");
        
        balanceOf[from] -= value;
        balanceOf[to] += value;
        allowance[from][msg.sender] -= value;
        
        emit Transfer(from, to, value);
        return true;
    }
    
    /**
     * @dev Mint new tokens (only owner)
     * @param to Address to receive the minted tokens
     * @param value Amount to mint (in wei, 18 decimals)
     */
    function mint(address to, uint256 value) public onlyOwner {
        _mint(to, value);
    }
    
    function _mint(address to, uint256 value) internal {
        require(to != address(0), "Cannot mint to zero address");
        totalSupply += value;
        balanceOf[to] += value;
        emit Mint(to, value);
        emit Transfer(address(0), to, value);
    }
}
