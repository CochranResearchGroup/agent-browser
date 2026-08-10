#!/usr/bin/env node

import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const repoRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../..");
const dispatchPath = path.join(repoRoot, "cli/src/native/action_runtime/dispatch.rs");
const actionsPath = path.join(repoRoot, "cli/src/native/actions.rs");
const runtimeRootPath = path.join(repoRoot, "cli/src/native/action_runtime.rs");
const commonPath = path.join(repoRoot, "cli/src/native/action_runtime/common.rs");
const testsPath = path.join(repoRoot, "cli/src/native/action_runtime/tests.rs");
const inventoryPath = path.join(
  repoRoot,
  "docs/dev/architecture/actions-responsibility-inventory.v1.json",
);
const retainedNames = [
  "action_skips_browser_launch",
  "cancellation_error",
  "cancellable",
  "active_target_binding",
  "handle_dependent_batch",
  "execute_command",
  "success_response",
  "error_response",
];

if (!fs.existsSync(dispatchPath)) {
  throw new Error(`missing dispatch recovery source: ${dispatchPath}`);
}

function matchingBrace(source, open) {
  let depth = 0;
  for (let index = open; index < source.length; index += 1) {
    if (source[index] === "{") depth += 1;
    if (source[index] === "}") {
      depth -= 1;
      if (depth === 0) return index + 1;
    }
  }
  throw new Error(`unbalanced function at byte ${open}`);
}

function takeFunction(source, name) {
  const match = new RegExp(`^pub\\(crate\\)\\s+(?:async\\s+)?fn\\s+${name}\\b`, "m").exec(source);
  if (!match) throw new Error(`missing function recovery source: ${name}`);
  let start = match.index;
  while (start > 0) {
    const previousEnd = start - 1;
    const previousStart = source.lastIndexOf("\n", previousEnd - 1) + 1;
    if (!source.slice(previousStart, previousEnd + 1).trim().startsWith("///")) break;
    start = previousStart;
  }
  const open = source.indexOf("{", match.index + match[0].length);
  let end = matchingBrace(source, open);
  if (source[end] === "\n") end += 1;
  return { text: source.slice(start, end), remaining: source.slice(0, start) + source.slice(end) };
}

let dispatch = fs.readFileSync(dispatchPath, "utf8");
for (const name of retainedNames.filter((name) => !["cancellation_error", "cancellable"].includes(name))) {
  const pattern = new RegExp(`\\b(?:async\\s+)?fn\\s+${name}\\b`);
  if (!pattern.test(dispatch)) throw new Error(`missing retained dispatcher definition: ${name}`);
}
let common = fs.readFileSync(commonPath, "utf8");
const cancellationError = takeFunction(common, "cancellation_error");
const cancellable = takeFunction(cancellationError.remaining, "cancellable");
common = cancellable.remaining;
dispatch = dispatch
  .replaceAll("super::super::", "super::")
  .replaceAll("use super::browser_operations::", "use super::action_runtime::browser_operations::")
  .replaceAll("use super::common::", "use super::action_runtime::common::")
  .replaceAll("use super::remote_view_operations::", "use super::action_runtime::remote_view_operations::")
  .replaceAll("use super::runtime::", "use super::action_runtime::runtime::")
  .replaceAll("use super::service_commands::", "use super::action_runtime::service_commands::")
  .replaceAll("use super::service_workflows::", "use super::action_runtime::service_workflows::");
dispatch += `\n${cancellationError.text.trim()}\n\n${cancellable.text.trim()}\n`;
fs.writeFileSync(actionsPath, dispatch);
fs.writeFileSync(commonPath, common);

for (const module of ["browser_operations.rs", "runtime.rs"]) {
  const modulePath = path.join(repoRoot, "cli/src/native/action_runtime", module);
  let source = fs.readFileSync(modulePath, "utf8");
  if (!source.includes("use crate::native::actions::cancellable;")) {
    source = source.replace(
      "use super::common::*;\n",
      "use super::common::*;\nuse crate::native::actions::cancellable;\n",
    );
  }
  fs.writeFileSync(modulePath, source);
}

let runtimeRoot = fs.readFileSync(runtimeRootPath, "utf8");
runtimeRoot = runtimeRoot
  .replace("mod dispatch;\n", "")
  .replace("pub(crate) use dispatch::{action_skips_browser_launch, execute_command};\n", "")
  .replace("#[cfg(test)]\n#[allow(unused_imports)]\npub(crate) use dispatch::*;\n", "");
fs.writeFileSync(runtimeRootPath, runtimeRoot);

let tests = fs.readFileSync(testsPath, "utf8");
if (!tests.includes("use crate::native::actions::*;")) {
  tests = tests.replace(
    "use super::*;\n",
    "use super::*;\nuse crate::native::actions::*;\n",
  );
}
fs.writeFileSync(testsPath, tests);

const inventory = JSON.parse(fs.readFileSync(inventoryPath, "utf8"));
const records = [
  ...inventory.definitions,
  ...(inventory.predecessorReconciliation?.addedDefinitions ?? []),
];
let moved = 0;
for (const record of records) {
  if (record.finalDisposition === "move" && record.movementStatus !== "moved") {
    record.movementStatus = "moved";
    moved += 1;
  }
}
const retained = records.filter((record) => record.finalDisposition === "retain");
if (
  retained.length !== retainedNames.length
  || retained.some((record) => !retainedNames.includes(record.name))
) {
  throw new Error("dispatcher allowlist differs from the eight retained inventory records");
}
fs.writeFileSync(inventoryPath, `${JSON.stringify(inventory, null, 2)}\n`);

const installedNames = retainedNames.filter((name) => {
  const pattern = new RegExp(`\\b(?:async\\s+)?fn\\s+${name}\\b`);
  return pattern.test(fs.readFileSync(actionsPath, "utf8"));
});
if (installedNames.length !== retainedNames.length) {
  throw new Error("actions dispatcher copy failed verification; recovery source preserved");
}
fs.unlinkSync(dispatchPath);
console.log(`retained=${retained.length} newly_moved=${moved} recovery_source_removed=true`);
