#!/usr/bin/env node

import fs from "node:fs";
import path from "node:path";
import { execFileSync } from "node:child_process";
import { fileURLToPath } from "node:url";

const repoRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../..");
const nativeRoot = path.join(repoRoot, "cli/src/native");
const sourceCommit = "940d2575";
const sourceRelative = "cli/src/native/action_runtime/service_workflows.rs";
const modules = [
  "service_probe",
  "service_ui_action",
  "service_network_capture",
  "service_file_transfer",
  "service_diagnostics",
];
const boundaryNames = [
  "handle_service_probe",
  "handle_service_ui_action",
  "handle_service_network_capture",
  "handle_service_file_transfer",
  "handle_service_diagnostics",
  "runtime_handoff_path",
];
const crossImports = new Map([
  ["service_probe", ["use super::service_diagnostics::truncate_utf8;"]],
  ["service_ui_action", [
    "use super::service_diagnostics::{handle_service_diagnostics, truncate_utf8};",
    "use super::service_probe::probe_recipe_fingerprint;",
  ]],
  ["service_network_capture", [
    "use super::service_diagnostics::truncate_utf8;",
    "use super::service_probe::probe_recipe_fingerprint;",
    "use super::service_ui_action::{service_ui_caller, service_ui_current_page};",
  ]],
  ["service_file_transfer", [
    "use super::service_diagnostics::handle_service_diagnostics;",
    "use super::service_probe::probe_recipe_fingerprint;",
    "use super::service_ui_action::{service_ui_caller, service_ui_current_page};",
  ]],
  ["service_diagnostics", []],
]);

function sourceAtCommit(relativePath) {
  return execFileSync("git", ["show", `${sourceCommit}:${relativePath}`], {
    cwd: repoRoot,
    encoding: "utf8",
    maxBuffer: 16 * 1024 * 1024,
  });
}

function exactBoundary(source, name) {
  const variants = [
    `pub(crate) async fn ${name}`,
    `pub(crate) fn ${name}`,
  ];
  for (const variant of variants) {
    const index = source.indexOf(variant);
    if (index >= 0 && (index === 0 || source[index - 1] === "\n")) return index;
  }
  throw new Error(`missing exact service workflow boundary: ${name}`);
}

function rootHeader(header) {
  return header
    .replace(/^#!\[allow\(unused_imports\)\]\n/, "")
    .replaceAll("super::browser_operations::", "super::action_runtime::browser_operations::")
    .replaceAll("super::common::", "super::action_runtime::common::")
    .replaceAll("super::runtime::", "super::action_runtime::runtime::");
}

const source = sourceAtCommit(sourceRelative);
const starts = boundaryNames.map((name) => exactBoundary(source, name));
for (let index = 1; index < starts.length; index += 1) {
  if (starts[index] <= starts[index - 1]) {
    throw new Error(`out-of-order service workflow boundary: ${boundaryNames[index]}`);
  }
}
const header = rootHeader(source.slice(0, starts[0]));
const bodies = new Map();
for (let index = 0; index < modules.length; index += 1) {
  bodies.set(modules[index], source.slice(starts[index], starts[index + 1]));
}
const lifecycleBody = source.slice(starts.at(-1));

for (const module of modules) {
  const imports = crossImports.get(module).join("\n");
  fs.writeFileSync(
    path.join(nativeRoot, `${module}.rs`),
    `#![allow(unused_imports)]\n${header}${imports}${imports ? "\n" : ""}${bodies.get(module)}`,
  );
}

const navigationRelative = "cli/src/native/action_runtime/runtime/navigation.rs";
const navigation = sourceAtCommit(navigationRelative)
  .replace(
    /use super::super::service_workflows::\{runtime_handoff_path, write_runtime_handoff\};\n/,
    "",
  );
fs.writeFileSync(path.join(repoRoot, navigationRelative), navigation + lifecycleBody);

const actionRuntimeRelative = "cli/src/native/action_runtime.rs";
const actionRuntime = sourceAtCommit(actionRuntimeRelative)
  .replace("pub(crate) mod service_workflows;\n", "")
  .replace("#[cfg(test)]\n#[allow(unused_imports)]\npub(crate) use service_workflows::*;\n", "");
fs.writeFileSync(path.join(repoRoot, actionRuntimeRelative), actionRuntime);

const actionsRelative = "cli/src/native/actions.rs";
const actions = sourceAtCommit(actionsRelative).replace(
  /use super::action_runtime::service_workflows::\{[\s\S]*?\};\n/,
  "use super::service_diagnostics::handle_service_diagnostics;\n" +
    "use super::service_file_transfer::handle_service_file_transfer;\n" +
    "use super::service_network_capture::handle_service_network_capture;\n" +
    "use super::service_probe::handle_service_probe;\n" +
    "use super::service_ui_action::handle_service_ui_action;\n",
);
fs.writeFileSync(path.join(repoRoot, actionsRelative), actions);

const browserOperationsRelative = "cli/src/native/action_runtime/browser_operations.rs";
const browserOperations = sourceAtCommit(browserOperationsRelative).replace(
  "use super::service_workflows::truncate_utf8;",
  "use super::super::service_diagnostics::truncate_utf8;",
);
fs.writeFileSync(path.join(repoRoot, browserOperationsRelative), browserOperations);

const testsRelative = "cli/src/native/action_runtime/tests.rs";
const tests = sourceAtCommit(testsRelative).replace(
  "use crate::native::service_health::{",
  modules.map((module) => `use crate::native::${module}::*;`).join("\n") +
    "\nuse crate::native::service_health::{",
);
fs.writeFileSync(path.join(repoRoot, testsRelative), tests);

const modRelative = "cli/src/native/mod.rs";
const marker = "#[allow(dead_code)]\npub mod service_health;\n";
const moduleDeclarations = modules
  .map((module) => `#[allow(dead_code)]\npub mod ${module};`)
  .join("\n") + "\n";
const nativeMod = sourceAtCommit(modRelative).replace(marker, `${moduleDeclarations}${marker}`);
fs.writeFileSync(path.join(repoRoot, modRelative), nativeMod);

const runtimeRoot = path.join(nativeRoot, "action_runtime/runtime");
for (const runtimeFile of fs.readdirSync(runtimeRoot)) {
  if (runtimeFile === "navigation.rs") continue;
  const relativePath = `cli/src/native/action_runtime/runtime/${runtimeFile}`;
  const runtime = sourceAtCommit(relativePath).replace(
    /use super::super::service_workflows::\{runtime_handoff_path, write_runtime_handoff\};\n/,
    "",
  );
  fs.writeFileSync(path.join(repoRoot, relativePath), runtime);
}

const itemBytes = source.length - starts[0];
const writtenBytes = [...bodies.values()].reduce((total, body) => total + body.length, 0) +
  lifecycleBody.length;
if (writtenBytes !== itemBytes) {
  throw new Error(`service workflow byte reconciliation failed: ${writtenBytes} != ${itemBytes}`);
}

const oldSourcePath = path.join(repoRoot, sourceRelative);
if (fs.existsSync(oldSourcePath)) fs.unlinkSync(oldSourcePath);
console.log(`service_workflow_modules=${modules.length} boundaries=${starts.length} reconciled_bytes=${writtenBytes}`);
