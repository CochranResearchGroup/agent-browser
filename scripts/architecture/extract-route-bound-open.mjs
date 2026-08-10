#!/usr/bin/env node

import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const repoRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../..");
const runtimeRoot = path.join(repoRoot, "cli/src/native/action_runtime");
const sourcePath = path.join(runtimeRoot, "remote_view_operations.rs");
const targetPath = path.join(repoRoot, "cli/src/native/remote_view/open.rs");
const inventory = JSON.parse(fs.readFileSync(
  path.join(repoRoot, "docs/dev/architecture/actions-responsibility-inventory.v1.json"),
  "utf8",
));
const openNames = new Set(inventory.definitions
  .filter((item) => item.targetModule === "native::remote_view::open")
  .map((item) => item.name));
openNames.add("RemoteViewOpenFailureCleanupInput");

function matchingBrace(source, open) {
  let depth = 0;
  let state = "code";
  for (let index = open; index < source.length; index += 1) {
    const current = source[index];
    const next = source[index + 1];
    if (state === "line-comment") {
      if (current === "\n") state = "code";
      continue;
    }
    if (state === "block-comment") {
      if (current === "*" && next === "/") {
        state = "code";
        index += 1;
      }
      continue;
    }
    if (state === "string") {
      if (current === "\\") index += 1;
      else if (current === '"') state = "code";
      continue;
    }
    if (state === "char") {
      if (current === "\\") index += 1;
      else if (current === "'") state = "code";
      continue;
    }
    if (current === "/" && next === "/") {
      state = "line-comment";
      index += 1;
      continue;
    }
    if (current === "/" && next === "*") {
      state = "block-comment";
      index += 1;
      continue;
    }
    if (current === '"') {
      state = "string";
      continue;
    }
    if (current === "'") {
      if (!/^'[A-Za-z_][A-Za-z0-9_]*\b/.test(source.slice(index, index + 16))) {
        state = "char";
      }
      continue;
    }
    if (current === "{") depth += 1;
    if (current === "}") {
      depth -= 1;
      if (depth === 0) return index + 1;
    }
  }
  throw new Error(`unbalanced item beginning at byte ${open}`);
}

function itemRanges(source) {
  const pattern = /^pub\(crate\)\s+(?:(?:async|unsafe)\s+)?(?:fn|struct)\s+([A-Za-z_][A-Za-z0-9_]*)\b/gm;
  const ranges = [];
  for (const match of source.matchAll(pattern)) {
    if (!openNames.has(match[1])) continue;
    let start = match.index;
    while (start > 0) {
      const previousEnd = start - 1;
      const previousStart = source.lastIndexOf("\n", previousEnd - 1) + 1;
      const previous = source.slice(previousStart, previousEnd + 1).trim();
      if (!previous.startsWith("///") && !previous.startsWith("#[")) break;
      start = previousStart;
    }
    const open = source.indexOf("{", match.index + match[0].length);
    if (open < 0) throw new Error(`missing body for ${match[1]}`);
    let end = matchingBrace(source, open);
    if (source[end] === "\n") end += 1;
    ranges.push({ name: match[1], start, end, text: source.slice(start, end) });
  }
  const found = new Set(ranges.map((item) => item.name));
  const missing = [...openNames].filter((name) => !found.has(name));
  if (missing.length > 0) throw new Error(`missing open items: ${missing.join(",")}`);
  return ranges.sort((left, right) => left.start - right.start);
}

function topLevelDeclarations(file) {
  const source = fs.readFileSync(file, "utf8");
  const pattern = /^pub\(crate\)\s+(?:(?:async|unsafe)\s+)?(?:fn|struct|enum|trait|type|const|static)\s+([A-Za-z_][A-Za-z0-9_]*)/gm;
  return [...source.matchAll(pattern)].map((match) => match[1]);
}

const source = fs.readFileSync(sourcePath, "utf8");
const ranges = itemRanges(source);
let remainder = source;
for (const range of [...ranges].reverse()) {
  remainder = remainder.slice(0, range.start) + remainder.slice(range.end);
}

const ownerFiles = fs.readdirSync(runtimeRoot)
  .filter((file) => file.endsWith(".rs") && file !== "tests.rs" && file !== "remote_view_operations.rs");
const owners = new Map();
for (const file of ownerFiles) {
  for (const name of topLevelDeclarations(path.join(runtimeRoot, file))) {
    owners.set(name, file.replace(/\.rs$/, ""));
  }
}
for (const name of topLevelDeclarations(sourcePath)) owners.set(name, "remote_view_operations");
for (const name of openNames) owners.set(name, "open");

const movedSource = ranges.map((item) => item.text.trimEnd()).join("\n\n");
const imports = new Map();
for (const [name, owner] of owners) {
  if (owner === "open" || owner === "common") continue;
  if (!new RegExp(`\\b${name}\\b`).test(movedSource)) continue;
  if (!imports.has(owner)) imports.set(owner, []);
  imports.get(owner).push(name);
}
const importText = [...imports]
  .sort(([left], [right]) => left.localeCompare(right))
  .map(([owner, names]) => {
    names.sort();
    return `use crate::native::action_runtime::${owner}::{${names.join(", ")}};`;
  })
  .join("\n");

fs.mkdirSync(path.dirname(targetPath), { recursive: true });
fs.writeFileSync(targetPath, `//! Route-bound browser acquisition and durable handoff resolution.\n\n#![allow(unused_imports)]\nuse crate::native::action_runtime::common::*;\n${importText}\n\n${movedSource}\n`);

const reverseImports = [];
for (const name of openNames) {
  if (new RegExp(`\\b${name}\\b`).test(remainder)) reverseImports.push(name);
}
if (reverseImports.length > 0) {
  const line = `use crate::native::remote_view::open::{${reverseImports.sort().join(", ")}};\n`;
  remainder = remainder.replace(/^use super::common::\*;\n/, (match) => `${match}${line}`);
}
fs.writeFileSync(sourcePath, remainder);

console.log(JSON.stringify({
  movedItems: ranges.length,
  movedNames: ranges.map((item) => item.name),
  openLines: fs.readFileSync(targetPath, "utf8").split("\n").length - 1,
  remainingLines: remainder.split("\n").length - 1,
  importGroups: imports.size,
  reverseImportCount: reverseImports.length,
}, null, 2));
