#!/usr/bin/env node

import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const repoRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../..");
const runtimeRoot = path.join(repoRoot, "cli/src/native/action_runtime");

function rewrite(file, transform) {
  const target = path.join(runtimeRoot, file);
  const before = fs.readFileSync(target, "utf8");
  const after = transform(before);
  if (before === after) throw new Error(`no rewrite applied to ${file}`);
  fs.writeFileSync(target, after);
}

rewrite("common.rs", (source) =>
  source.replace(/^pub\(crate\) use super::/gm, "pub(crate) use super::super::"),
);
rewrite("browser_operations.rs", (source) =>
  source.replace(/use super::element::resolve_element_object_id;/g,
    "use super::super::element::resolve_element_object_id;"),
);
rewrite("dispatch.rs", (source) =>
  source.replace(/super::dependent_batch::TargetEffect::Rebind/g,
    "super::super::dependent_batch::TargetEffect::Rebind"),
);

console.log("fixed action-runtime native-relative paths");
