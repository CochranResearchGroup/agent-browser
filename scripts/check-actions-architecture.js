#!/usr/bin/env node

import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import path from "node:path";

const repoRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const manifest = path.join(repoRoot, "scripts/architecture/actions-inventory/Cargo.toml");
const source = path.join(repoRoot, "cli/src/native/actions.rs");
const inventory = path.join(repoRoot, "docs/dev/architecture/actions-responsibility-inventory.v1.json");
const fixtures = path.join(repoRoot, "scripts/architecture/actions-inventory/fixtures");
const cargoSafe = path.join(repoRoot, "scripts/ci/cargo-safe.sh");
const mode = process.argv.includes("--self-test") ? "self-test" : "check";
const commandArgs = mode === "self-test"
  ? ["run", "--quiet", "--bin", "actions-inventory", "--manifest-path", manifest, "--", "self-test", "--fixtures", fixtures]
  : ["run", "--quiet", "--bin", "actions-inventory", "--manifest-path", manifest, "--", "check", "--source", source, "--inventory", inventory];
const result = spawnSync(cargoSafe, commandArgs, { cwd: repoRoot, stdio: "inherit" });
if (result.error) {
  console.error(`actions architecture checker failed to start: ${result.error.message}`);
  process.exit(1);
}
process.exit(result.status ?? 1);
