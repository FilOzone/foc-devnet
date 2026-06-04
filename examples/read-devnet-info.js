/**
 * FOC DevNet Info Reader
 *
 * This example demonstrates how to read and use the devnet-info.json file
 * exported by foc-devnet after a successful start.
 *
 * Usage:
 *   node read-devnet-info.js [path-to-devnet-info.json]
 *
 * If no path is provided, defaults to ~/.foc-devnet/state/latest/devnet-info.json
 */

import { readFileSync, existsSync } from "fs";
import { homedir } from "os";
import { join } from "path";
import { validateDevnetInfo } from "./devnet-schema.js";

/**
 * Load and validate the devnet-info.json file.
 * @param {string} filePath - Path to the devnet-info.json file
 * @returns {object} The parsed and validated DevNet info
 */
function loadDevnetInfo(filePath) {
  if (!existsSync(filePath)) {
    throw new Error(`DevNet info file not found: ${filePath}`);
  }

  const content = readFileSync(filePath, "utf8");
  const data = JSON.parse(content);

  // Validate against schema - this will throw if invalid
  return validateDevnetInfo(data);
}

/**
 * Print summary of the DevNet state.
 * @param {object} info - The DevnetInfoV1 object
 */
function printSummary(info) {
  console.log("\n═══════════════════════════════════════════════════════════");
  console.log("                    FOC DevNet Summary");
  console.log("═══════════════════════════════════════════════════════════\n");

  console.log(`Run ID:           ${info.run_id}`);
  console.log(`Start Time:       ${info.start_time}`);
  console.log(`Startup Duration: ${info.startup_duration}`);
}

/**
 * Print Lotus node information.
 * @param {object} lotus - The LotusInfo object
 */
function printLotusInfo(lotus) {
  console.log("\n─── Lotus Node ───────────────────────────────────────────\n");
  console.log(`RPC URL:        ${lotus.host_rpc_url}`);
  console.log(`Container:      ${lotus.container_name}`);
  console.log(`Container ID:   ${lotus.container_id.substring(0, 12)}...`);
}

/**
 * Print deployed contract addresses.
 * @param {object} contracts - The ContractsInfo object
 */
function printContracts(contracts) {
  console.log("\n─── Deployed Contracts ───────────────────────────────────\n");
  console.log(`MockUSDFC:                   ${contracts.mockusdfc_addr}`);
  console.log(`Multicall3:                  ${contracts.multicall3_addr}`);
  console.log(`FWSS Proxy:                  ${contracts.fwss_service_proxy_addr}`);
  console.log(`PDP Verifier Proxy:          ${contracts.pdp_verifier_proxy_addr}`);
  console.log(`Service Provider Registry:   ${contracts.service_provider_registry_proxy_addr}`);
  console.log(`FilecoinPay V1:              ${contracts.filecoin_pay_v1_addr}`);
  console.log(`Endorsements:                ${contracts.endorsements_addr}`);
}

/**
 * Print user account information.
 * @param {Array} users - Array of UserInfo objects
 */
function printUsers(users) {
  console.log("\n─── User Accounts ────────────────────────────────────────\n");

  for (const user of users) {
    console.log(`${user.name}:`);
    console.log(`  EVM Address:    ${user.evm_addr}`);
    console.log(`  Native Address: ${user.native_addr}`);
    console.log(`  Private Key:    ${user.private_key_hex.substring(0, 10)}...`);
    console.log();
  }
}

/**
 * Print PDP service provider information.
 * @param {Array} providers - Array of CurioInfo objects
 */
function printCurioProviders(providers) {
  console.log("\n─── PDP Service Providers ────────────────────────────────\n");

  for (const provider of providers) {
    console.log(`Provider ${provider.provider_id}:`);
    console.log(`  ETH Address:      ${provider.eth_addr}`);
    console.log(`  PDP Service URL:  ${provider.pdp_service_url}`);
    console.log(`  Container:        ${provider.container_name}`);
    console.log(`  Container ID:     ${provider.container_id.substring(0, 12)}...`);
    console.log(`  Databases:`);
    console.log(`    Postgres Port:  ${provider.database.postgres_port}`);
    console.log(`    Scylla Port:    ${provider.database.scylla_port}`);
    console.log();
  }
}

// Main execution
function main() {
  // Determine file path
  const defaultPath = join(
    homedir(),
    ".foc-devnet",
    "state",
    "latest",
    "devnet-info.json"
  );
  const filePath = process.argv[2] || defaultPath;

  console.log(`Loading DevNet info from: ${filePath}`);

  try {
    const { version, info } = loadDevnetInfo(filePath);
    console.log(`Schema version: ${version}`);

    printSummary(info);
    printLotusInfo(info.lotus);
    printContracts(info.contracts);
    printUsers(info.users);
    printCurioProviders(info.pdp_sps);

    console.log("═══════════════════════════════════════════════════════════\n");
  } catch (error) {
    console.error(`Error: ${error.message}`);
    process.exit(1);
  }
}

main();
