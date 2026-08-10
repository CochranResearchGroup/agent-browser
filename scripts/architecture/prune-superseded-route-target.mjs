#!/usr/bin/env node

import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const repoRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../..");
const targetPath = path.join(repoRoot, "cli/src/native/remote_view/open/target.rs");
const names = ["remote_view_open_acquire_tab", "remote_view_open_wait_for_target_url"];

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
    if (current === "/" && next === "/") {
      state = "line-comment";
      index += 1;
    } else if (current === "/" && next === "*") {
      state = "block-comment";
      index += 1;
    } else if (current === '"') {
      state = "string";
    } else if (current === "{") {
      depth += 1;
    } else if (current === "}" && --depth === 0) {
      return index + 1;
    }
  }
  throw new Error(`unbalanced function body at byte ${open}`);
}

function functionRange(source, name) {
  const match = new RegExp(
    `^pub\\(crate\\)\\s+(?:async\\s+)?fn\\s+${name}\\b`,
    "m",
  ).exec(source);
  if (!match) throw new Error(`missing superseded function: ${name}`);
  let start = match.index;
  while (start > 0) {
    const previousEnd = start - 1;
    const previousStart = source.lastIndexOf("\n", previousEnd - 1) + 1;
    const previous = source.slice(previousStart, previousEnd + 1).trim();
    if (!previous.startsWith("///") && !previous.startsWith("#[")) break;
    start = previousStart;
  }
  const open = source.indexOf("{", match.index + match[0].length);
  let end = matchingBrace(source, open);
  if (source[end] === "\n") end += 1;
  return { start, end };
}

let source = fs.readFileSync(targetPath, "utf8");
let removedBytes = 0;
for (const name of names) {
  const range = functionRange(source, name);
  removedBytes += range.end - range.start;
  source = source.slice(0, range.start) + source.slice(range.end);
}
for (const name of names) {
  if (new RegExp(`\\b${name}\\b`).test(source)) {
    throw new Error(`superseded reference survived pruning: ${name}`);
  }
}
fs.writeFileSync(targetPath, source);
console.log(`removed_functions=${names.length} removed_bytes=${removedBytes}`);
