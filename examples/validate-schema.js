#!/usr/bin/env node
/**
 * Schema validation script for DevNet info export.
 * Used by CI to validate the exported devnet-info.json file.
 */

import { readFileSync } from "fs";
import { validateDevnetInfo } from "./devnet-schema.js";

const devnetInfoPath = process.argv[2];

if (!devnetInfoPath) {
  console.error("Usage: node validate-schema.js <path-to-devnet-info.json>");
  process.exit(1);
}

try {
  const content = readFileSync(devnetInfoPath, "utf8");
  const data = JSON.parse(content);
  
  validateDevnetInfo(data);
  console.log("✓ Schema validation passed");
  process.exit(0);
} catch (error) {
  console.error("✗ Schema validation failed:");
  console.error(error.message);
  process.exit(1);
}
