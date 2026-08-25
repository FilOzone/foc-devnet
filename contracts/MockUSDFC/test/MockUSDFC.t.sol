// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import "forge-std/Test.sol";
import "../src/MockUSDFC.sol";

contract MockUSDFCTest is Test {
    bytes32 private constant PERMIT_TYPEHASH =
        keccak256("Permit(address owner,address spender,uint256 value,uint256 nonce,uint256 deadline)");

    MockUSDFC private token;
    uint256 private ownerKey;
    address private owner;

    function setUp() public {
        token = new MockUSDFC(0);
        ownerKey = 0xA11CE;
        owner = vm.addr(ownerKey);
        token.mint(owner, 100 ether);
    }

    function testPermit() public {
        address spender = makeAddr("spender");
        uint256 value = 25 ether;
        uint256 deadline = block.timestamp + 1 hours;
        bytes32 structHash = keccak256(abi.encode(PERMIT_TYPEHASH, owner, spender, value, 0, deadline));
        bytes32 digest = keccak256(abi.encodePacked("\x19\x01", token.DOMAIN_SEPARATOR(), structHash));
        (uint8 v, bytes32 r, bytes32 s) = vm.sign(ownerKey, digest);

        token.permit(owner, spender, value, deadline, v, r, s);

        assertEq(token.allowance(owner, spender), value);
        assertEq(token.nonces(owner), 1);
        assertEq(token.version(), "1");
    }
}
