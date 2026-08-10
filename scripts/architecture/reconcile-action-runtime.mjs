#!/usr/bin/env node

import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const repoRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../..");
const runtimeRoot = path.join(repoRoot, "cli/src/native/action_runtime");
const parentPath = path.join(repoRoot, "cli/src/native/action_runtime.rs");
const modules = [
  "browser_operations",
  "common",
  "remote_view_operations",
  "runtime",
  "service_commands",
  "service_workflows",
];
const dispatchNames = new Set([
  "action_skips_browser_launch",
  "cancellation_error",
  "cancellable",
  "active_target_binding",
  "handle_dependent_batch",
  "execute_command",
  "success_response",
  "error_response",
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
      const tail = source.slice(index, index + 16);
      if (!/^'[A-Za-z_][A-Za-z0-9_]*\b/.test(tail)) state = "char";
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

function itemRange(source, name) {
  const expression = new RegExp(
    `^pub\\(crate\\)\\s+(?:(?:async|unsafe)\\s+)?fn\\s+${name}\\b`,
    "m",
  );
  const match = expression.exec(source);
  if (!match) throw new Error(`missing dispatch item ${name}`);
  let start = match.index;
  while (start > 0) {
    const previousEnd = start - 1;
    const previousStart = source.lastIndexOf("\n", previousEnd - 1) + 1;
    const previous = source.slice(previousStart, previousEnd + 1).trim();
    if (!previous.startsWith("///") && !previous.startsWith("#[")) break;
    start = previousStart;
  }
  const open = source.indexOf("{", match.index + match[0].length);
  if (open < 0) throw new Error(`missing body for dispatch item ${name}`);
  let end = matchingBrace(source, open);
  if (source[end] === "\n") end += 1;
  return { start, end, text: source.slice(start, end) };
}

function removeLocalWildcardImports(source) {
  return source.replace(
    /^pub\(crate\) use super::(?:browser_operations|common|remote_view_operations|runtime|service_commands|service_workflows)::\*;\n/gm,
    "",
  );
}

const sources = Object.fromEntries(
  modules.map((module) => [
    module,
    removeLocalWildcardImports(
      fs.readFileSync(path.join(runtimeRoot, `${module}.rs`), "utf8"),
    ),
  ]),
);
const moved = [];
for (const name of dispatchNames) {
  const owner = modules.find((module) => {
    const expression = new RegExp(
      `^pub\\(crate\\)\\s+(?:(?:async|unsafe)\\s+)?fn\\s+${name}\\b`,
      "m",
    );
    return expression.test(sources[module]);
  });
  if (!owner) throw new Error(`dispatch owner not found for ${name}`);
  const range = itemRange(sources[owner], name);
  moved.push({ name, owner, text: range.text });
  sources[owner] = sources[owner].slice(0, range.start) + sources[owner].slice(range.end);
}

const declarationPattern = /^pub\(crate\)\s+(?:(?:async|unsafe)\s+)?(?:fn|struct|enum|trait|type|const|static)\s+([A-Za-z_][A-Za-z0-9_]*)/gm;
const declarations = new Map();
for (const module of modules) {
  for (const match of sources[module].matchAll(declarationPattern)) {
    if (declarations.has(match[1])) {
      throw new Error(`duplicate top-level declaration ${match[1]}`);
    }
    declarations.set(match[1], module);
  }
}
for (const { name } of moved) declarations.set(name, "dispatch");

function explicitImports(module, source) {
  const imports = new Map();
  for (const [name, owner] of declarations) {
    if (owner === module || owner === "common") continue;
    if (!new RegExp(`\\b${name}\\b`).test(source)) continue;
    if (!imports.has(owner)) imports.set(owner, []);
    imports.get(owner).push(name);
  }
  return [...imports]
    .sort(([left], [right]) => left.localeCompare(right))
    .map(([owner, names]) => {
      names.sort();
      return `use super::${owner}::{${names.join(", ")}};`;
    })
    .join("\n");
}

for (const module of modules) {
  let source = sources[module].replace(/^#!\[allow\(unused_imports\)\]\n/, "");
  const imports = explicitImports(module, source);
  const prelude = module === "common" ? "" : "use super::common::*;\n";
  source = `#![allow(unused_imports)]\n${prelude}${imports}${imports ? "\n" : ""}${source}`;
  fs.writeFileSync(path.join(runtimeRoot, `${module}.rs`), source);
}

let dispatch = moved
  .sort((left, right) => [...dispatchNames].indexOf(left.name) - [...dispatchNames].indexOf(right.name))
  .map(({ text }) => text.trimEnd())
  .join("\n\n");
dispatch = `//! Serialized command routing, shared gates, timing, and response envelopes.\n\n#![allow(unused_imports)]\nuse super::common::*;\n${explicitImports("dispatch", dispatch)}\n\n${dispatch}\n`;
fs.writeFileSync(path.join(runtimeRoot, "dispatch.rs"), dispatch);

const parent = `mod browser_operations;
mod common;
mod dispatch;
mod remote_view_operations;
mod runtime;
mod service_commands;
mod service_workflows;

pub(crate) use dispatch::{action_skips_browser_launch, execute_command};
pub(crate) use runtime::{DaemonState, ServiceProfileLeaseGate, TrackedRequest, service_profile_lease_gate};
pub(crate) use service_commands::{handle_service_status, handle_service_status_with_dependencies};
pub(crate) use remote_view_operations::refresh_cdp_screencast_view_streams;
pub(crate) use browser_operations::matches_status_filter;

#[cfg(test)]
pub(crate) use browser_operations::*;
#[cfg(test)]
pub(crate) use dispatch::*;
#[cfg(test)]
pub(crate) use remote_view_operations::*;
#[cfg(test)]
pub(crate) use runtime::*;
#[cfg(test)]
pub(crate) use service_commands::*;
#[cfg(test)]
pub(crate) use service_workflows::*;

#[cfg(test)]
mod tests;
`;
fs.writeFileSync(parentPath, parent);

const importCounts = {};
for (const module of [...modules, "dispatch"]) {
  const source = fs.readFileSync(path.join(runtimeRoot, `${module}.rs`), "utf8");
  importCounts[module] = (source.match(/^use super::[a-z_]+::\{/gm) || []).length;
}
console.log(JSON.stringify({
  movedDispatchItems: moved.map(({ name, owner }) => ({ name, owner })),
  moduleLines: Object.fromEntries([...modules, "dispatch"].map((module) => [
    module,
    fs.readFileSync(path.join(runtimeRoot, `${module}.rs`), "utf8").split("\n").length - 1,
  ])),
  explicitCrossOwnerImportGroups: importCounts,
}, null, 2));
