import { readFileSync } from "node:fs";
import { CID } from "multiformats/cid";
import { sha256 } from "multiformats/hashes/sha2";

const [cidString, filePath] = process.argv.slice(2);

if (!cidString || !filePath) {
  console.error("usage: node verify.mjs <cid> <file>");
  process.exit(1);
}

const cid = CID.parse(cidString);
const bytes = readFileSync(filePath);
const digest = await sha256.digest(bytes);

if (Buffer.from(digest.digest).equals(Buffer.from(cid.multihash.digest))) {
  console.log("OK");
} else {
  console.error("CID mismatch");
  process.exit(1);
}
