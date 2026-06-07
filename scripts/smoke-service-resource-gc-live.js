#!/usr/bin/env node

import { execFileSync, spawn } from "node:child_process";

const BIN = "./cli/target/debug/agent-browser";

function runJson(args) {
  const raw = execFileSync(BIN, args, { encoding: "utf8" });
  const parsed = JSON.parse(raw);
  if (!parsed.success) {
    throw new Error(parsed.error || `${BIN} ${args.join(" ")} failed`);
  }
  return parsed.data;
}

function pidRunning(pid) {
  try {
    process.kill(pid, 0);
    return true;
  } catch {
    return false;
  }
}

function candidatePids(data) {
  return (data.actions?.terminateProcess ?? [])
    .map((candidate) => candidate.pid)
    .filter((pid) => Number.isInteger(pid));
}

function sleep(ms) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

async function main() {
  try {
    execFileSync("which", ["Xvfb"], { stdio: "ignore" });
  } catch {
    console.log("service-resource-gc-live: skipped; Xvfb is not installed");
    return;
  }

  const before = runJson(["service", "gc", "--dry-run", "--json"]);
  const beforePids = candidatePids(before);
  if (beforePids.length > 0) {
    throw new Error(
      `Refusing live GC smoke because pre-existing candidates are present: ${beforePids.join(", ")}`,
    );
  }

  const display = `:${250 + Math.floor(Math.random() * 200)}`;
  const child = spawn("Xvfb", [display, "-screen", "0", "64x64x24", "-nolisten", "tcp"], {
    stdio: "ignore",
  });
  let cleaned = false;
  const cleanup = () => {
    if (!cleaned && child.pid && pidRunning(child.pid)) {
      try {
        process.kill(child.pid, "SIGKILL");
      } catch {
        // Best effort cleanup after failed smoke.
      }
    }
    cleaned = true;
  };
  process.on("exit", cleanup);
  process.on("SIGINT", () => {
    cleanup();
    process.exit(130);
  });

  await sleep(500);
  if (!child.pid || !pidRunning(child.pid)) {
    throw new Error("Generated Xvfb process did not stay alive for GC smoke");
  }

  const dryRun = runJson(["service", "gc", "--dry-run", "--json"]);
  const pids = candidatePids(dryRun);
  if (pids.length !== 1 || pids[0] !== child.pid) {
    throw new Error(
      `Expected generated Xvfb PID ${child.pid} as the only candidate; got ${pids.join(", ") || "none"}`,
    );
  }
  if (!dryRun.reviewToken) {
    throw new Error("Dry-run did not return a review token");
  }

  const apply = runJson(["service", "gc", "--apply", "--review-token", dryRun.reviewToken, "--json"]);
  const counts = apply.counts ?? {};
  if (counts.terminated !== 1 || counts.skipped !== 0 || counts.failed !== 0) {
    throw new Error(`Unexpected apply counts: ${JSON.stringify(counts)}`);
  }
  await sleep(500);
  if (pidRunning(child.pid)) {
    throw new Error(`Generated Xvfb PID ${child.pid} survived GC apply`);
  }
  cleaned = true;

  const after = runJson(["service", "gc", "--dry-run", "--json"]);
  const afterPids = candidatePids(after);
  if (afterPids.includes(child.pid)) {
    throw new Error(`Generated Xvfb PID ${child.pid} is still listed as a candidate`);
  }

  console.log(
    `service-resource-gc-live: ok terminated_pid=${child.pid} display=${display} after_candidates=${afterPids.length}`,
  );
}

main().catch((err) => {
  console.error(`service-resource-gc-live: ${err.message}`);
  process.exit(1);
});
