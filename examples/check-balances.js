/**
 * FOC DevNet Balance Checker
 *
 * This example demonstrates how to use ethers.js to check balances
 * of user accounts on the DevNet using the exported devnet-info.json.
 *
 * Usage:
 *   node check-balances.js [path-to-devnet-info.json]
 *
 * If no path is provided, defaults to ~/.foc-devnet/state/latest/devnet-info.json
 */

import { readFileSync, existsSync } from "fs";
import { homedir } from "os";
import { join } from "path";
import { createPublicClient, http, parseAbi } from "viem";
import { formatUnits } from "viem";

// ERC20 ABI (minimal for balanceOf)
const ERC20_ABI = parseAbi([
  "function balanceOf(address owner) view returns (uint256)",
  "function decimals() view returns (uint8)",
  "function symbol() view returns (string)",
]);

/**
 * Load the devnet-info.json file.
 * @param {string} filePath - Path to the devnet-info.json file
 * @returns {object} The parsed DevNet info
 */
function loadDevnetInfo(filePath) {
  if (!existsSync(filePath)) {
    throw new Error(`DevNet info file not found: ${filePath}`);
  }
  const content = readFileSync(filePath, "utf8");
  return JSON.parse(content);
}

/**
 * Format wei to ether with specified decimals.
 * @param {bigint} wei - Amount in wei
 * @param {number} decimals - Decimal places
 * @returns {string} Formatted amount
 */
function formatBalance(wei, decimals = 18) {
  return formatUnits(wei, decimals);
}

/**
 * Check native FIL balance for an address.
 * @param {object} client - Viem public client
 * @param {string} address - Address to check
 * @returns {Promise<string>} Balance in FIL
 */
async function checkNativeBalance(client, address) {
  const balance = await client.getBalance({ address });
  return formatBalance(balance);
}

/**
 * Check ERC20 token balance for an address.
 * @param {object} client - Viem public client
 * @param {string} tokenAddress - Token contract address
 * @param {string} userAddress - User address to check
 * @returns {Promise<{balance: string, symbol: string}>} Balance and symbol
 */
async function checkTokenBalance(client, tokenAddress, userAddress) {
  const [balance, decimals, symbol] = await Promise.all([
    client.readContract({
      address: tokenAddress,
      abi: ERC20_ABI,
      functionName: "balanceOf",
      args: [userAddress],
    }),
    client.readContract({
      address: tokenAddress,
      abi: ERC20_ABI,
      functionName: "decimals",
    }),
    client.readContract({
      address: tokenAddress,
      abi: ERC20_ABI,
      functionName: "symbol",
    }),
  ]);
  return {
    balance: formatBalance(balance, decimals),
    symbol,
  };
}

// Main execution
async function main() {
  // Determine file path
  const defaultPath = join(
    homedir(),
    ".foc-devnet",
    "state",
    "latest",
    "devnet-info.json"
  );
  const filePath = process.argv[2] || defaultPath;

  console.log(`Loading DevNet info from: ${filePath}\n`);

  try {
    const { info } = loadDevnetInfo(filePath);

    // Connect to the Lotus RPC
    const client = createPublicClient({
      transport: http(info.lotus.host_rpc_url),
    });
    console.log(`Connected to: ${info.lotus.host_rpc_url}`);

    // Check if network is accessible
    const blockNumber = await client.getBlockNumber();
    console.log(`Current block: ${blockNumber}\n`);

    console.log("═══════════════════════════════════════════════════════════");
    console.log("                    Account Balances");
    console.log("═══════════════════════════════════════════════════════════\n");

    // Check balances for all users
    for (const user of info.users) {
      console.log(`${user.name} (${user.evm_addr}):`);

      // Check native FIL balance
      const filBalance = await checkNativeBalance(client, user.evm_addr);
      console.log(`  Native FIL:  ${filBalance} tFIL`);

      // Check MockUSDFC balance
      if (info.contracts.mockusdfc_addr) {
        try {
          const { balance, symbol } = await checkTokenBalance(
            client,
            info.contracts.mockusdfc_addr,
            user.evm_addr
          );
          console.log(`  ${symbol}:    ${balance}`);
        } catch (e) {
          console.log(`  MockUSDFC:   Error - ${e.message}`);
        }
      }
      console.log();
    }

    console.log("═══════════════════════════════════════════════════════════\n");
  } catch (error) {
    console.error(`Error: ${error.message}`);
    if (error.code === "ECONNREFUSED") {
      console.error(
        "Could not connect to the DevNet. Make sure foc-devnet is running."
      );
    }
    process.exit(1);
  }
}

main();
