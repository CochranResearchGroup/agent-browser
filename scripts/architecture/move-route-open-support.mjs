#!/usr/bin/env node

import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const repoRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../..");
const sourcePath = path.join(repoRoot, "cli/src/native/action_runtime/remote_view_operations.rs");
const targetPath = path.join(repoRoot, "cli/src/native/remote_view/open.rs");
const seeds = new Set([
  "merge_route_into_checkout",
  "merge_route_pool_entry_into_checkout",
  "merge_stream_into_checkout",
  "remote_view_open_acquire_tab",
  "retained_readiness_component",
  "select_browser_reattach_route_pool_entry",
]);

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
  throw new Error(`unbalanced item at ${open}`);
}

function ranges(source) {
  const pattern = /^pub\(crate\)\s+(?:(?:async|unsafe)\s+)?(?:fn|struct|enum|type)\s+([A-Za-z_][A-Za-z0-9_]*)\b/gm;
  const result = [];
  for (const match of source.matchAll(pattern)) {
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
    result.push({ name: match[1], start, end, text: source.slice(start, end) });
  }
  return result;
}

const source = fs.readFileSync(sourcePath, "utf8");
const items = ranges(source);
const byName = new Map(items.map((item) => [item.name, item]));
const selected = new Set(seeds);
for (const seed of seeds) {
  if (!byName.has(seed)) throw new Error(`missing route support seed ${seed}`);
}
let changed = true;
while (changed) {
  changed = false;
  const text = [...selected].map((name) => byName.get(name).text).join("\n");
  for (const item of items) {
    if (selected.has(item.name)) continue;
    if (new RegExp(`\\b${item.name}\\b`).test(text)) {
      selected.add(item.name);
      changed = true;
    }
  }
}
const moved = items.filter((item) => selected.has(item.name));
let remainder = source;
for (const item of [...moved].sort((left, right) => right.start - left.start)) {
  remainder = remainder.slice(0, item.start) + remainder.slice(item.end);
}
let target = fs.readFileSync(targetPath, "utf8");
target = target.replace(/^use crate::native::action_runtime::remote_view_operations::\{[^\n]+\};\n/m, "");
target += `\n${moved.map((item) => item.text.trimEnd()).join("\n\n")}\n`;
fs.writeFileSync(sourcePath, remainder);
fs.writeFileSync(targetPath, target);
console.log(JSON.stringify({
  movedCount: moved.length,
  movedNames: moved.map((item) => item.name),
  remainingLines: remainder.split("\n").length - 1,
  openLines: target.split("\n").length - 1,
}, null, 2));
