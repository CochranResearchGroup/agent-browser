#!/usr/bin/env node

import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const repoRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../..");
const sourcePath = path.join(repoRoot, "cli/src/native/action_runtime/runtime.rs");
const outputRoot = path.join(repoRoot, "cli/src/native/action_runtime/runtime");
const boundaries = new Map([
  ["AUTH_LOGIN_WAIT_UNTIL", "daemon"],
  ["browser_capability_service_state", "capability"],
  ["CdpFreeLaunchPlan", "cdp_free_plan"],
  ["remote_headed_view_streams_from_command", "remote_headed"],
  ["service_profile_lease_gate", "profile_lease"],
  ["persist_browser_recovery_started_from_persisted_state", "recovery"],
  ["shared_profile_auto_launch_acquisition_evidence", "launch"],
  ["build_cdp_free_launch_plan", "cdp_free_execute"],
  ["handle_navigate", "navigation"],
]);
const modules = [...new Set(boundaries.values())];

function expandedStart(source, index) {
  let start = index;
  while (start > 0) {
    const previousEnd = start - 1;
    const previousStart = source.lastIndexOf("\n", previousEnd - 1) + 1;
    const previous = source.slice(previousStart, previousEnd + 1).trim();
    if (!previous.startsWith("///") && !previous.startsWith("#[")) break;
    start = previousStart;
  }
  return start;
}

function declarationName(chunk) {
  const named = /^(?:pub\(crate\)\s+)?(?:async\s+)?(?:const|static|type|struct|enum|trait|fn)\s+([A-Za-z_][A-Za-z0-9_]*)/m.exec(chunk);
  return named?.[1];
}

function rewriteHeader(header) {
  return header
    .replace(/^#!\[allow\(unused_imports\)\]\n/, "")
    .replace("use crate::native::actions::cancellable;\n", "")
    .replaceAll("super::super::", "super::super::super::")
    .replaceAll("super::browser_operations::", "super::super::browser_operations::")
    .replaceAll("super::common::", "super::super::common::")
    .replaceAll("super::service_workflows::", "super::super::service_workflows::");
}

function rewriteBody(body) {
  return body
    .replaceAll("super::super::browser::", "super::super::super::browser::")
    .replaceAll("super::super::webdriver::", "super::super::super::webdriver::");
}

const source = fs.readFileSync(sourcePath, "utf8");
const itemPattern = /^(?:pub\(crate\)\s+)?(?:async\s+)?(?:const|static|type|struct|enum|trait|fn)\s+[A-Za-z_][A-Za-z0-9_]*|^impl(?:<[^\n]*>)?\s+/gm;
const starts = [];
for (const match of source.matchAll(itemPattern)) {
  const start = expandedStart(source, match.index);
  if (starts.at(-1) !== start) starts.push(start);
}
if (starts.length === 0) throw new Error("runtime splitter found no top-level items");
const header = rewriteHeader(source.slice(0, starts[0]));
const chunks = starts.map((start, index) => ({
  start,
  text: source.slice(start, starts[index + 1] ?? source.length),
}));

let currentModule;
const grouped = new Map(modules.map((module) => [module, []]));
const declarations = new Map();
for (const chunk of chunks) {
  const name = declarationName(chunk.text);
  if (name && boundaries.has(name)) currentModule = boundaries.get(name);
  if (!currentModule) throw new Error(`runtime item precedes first boundary: ${name ?? "impl"}`);
  grouped.get(currentModule).push(chunk.text);
  if (name) {
    if (declarations.has(name)) throw new Error(`duplicate runtime declaration: ${name}`);
    declarations.set(name, currentModule);
  }
}

fs.mkdirSync(outputRoot, { recursive: true });
for (const module of modules) {
  const body = rewriteBody(grouped.get(module).join(""));
  const imports = new Map();
  for (const [name, owner] of declarations) {
    if (owner === module || !new RegExp(`\\b${name}\\b`).test(body)) continue;
    if (!imports.has(owner)) imports.set(owner, []);
    imports.get(owner).push(name);
  }
  const siblingImports = [...imports]
    .sort(([left], [right]) => left.localeCompare(right))
    .map(([owner, names]) => `use super::${owner}::{${names.sort().join(", ")}};`)
    .join("\n");
  fs.writeFileSync(
    path.join(outputRoot, `${module}.rs`),
    `#![allow(unused_imports)]\n${header}${siblingImports}${siblingImports ? "\n" : ""}${body}`,
  );
}
const root = modules
  .map((module) => `mod ${module};\npub(crate) use ${module}::*;`)
  .join("\n");
fs.writeFileSync(
  sourcePath,
  `//! Daemon runtime, browser lifecycle, and launch coordination.\n\n#![allow(unused_imports)]\n${root}\n`,
);
const writtenBytes = modules.reduce(
  (total, module) => total + grouped.get(module).join("").length,
  0,
);
const itemBytes = source.length - starts[0];
if (writtenBytes !== itemBytes) {
  throw new Error(`runtime item byte reconciliation failed: ${writtenBytes} != ${itemBytes}`);
}
console.log(`runtime_modules=${modules.length} items=${chunks.length} reconciled_bytes=${writtenBytes}`);
