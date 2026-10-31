#!/usr/bin/env node

import { execFileSync, spawnSync } from 'node:child_process';
import { closeSync, mkdirSync, openSync, readFileSync, renameSync, writeFileSync } from 'node:fs';
import { join, resolve } from 'node:path';
import { PLAN_SCHEMA, runBoundValidation } from '../lib/local-validation.mjs';

const usage = 'Usage: pnpm validation:run --plan <checks.json> --output-dir <new-directory>';

let boundPlan = false;
try {
  const args = process.argv.slice(2);
  if (args.length === 1 && args[0] === '--help') {
    console.log(usage);
  } else {
    const options = new Map();
    for (let i = 0; i < args.length; i += 2) {
      if (!['--plan', '--output-dir'].includes(args[i]) || !args[i + 1] || options.has(args[i])) {
        throw new Error(usage);
      }
      options.set(args[i], args[i + 1]);
    }
    if (options.size !== 2) throw new Error(usage);
    const plan = JSON.parse(readFileSync(resolve(options.get('--plan')), 'utf8'));
    if (plan.schemaVersion === PLAN_SCHEMA) {
      boundPlan = true;
      const receipt = await runBoundValidation(plan, resolve(options.get('--output-dir')));
      process.exitCode = receipt.state === 'success' ? 0 : 1;
    } else {
    validatePlan(plan);
    const source = {
      head: execFileSync('git', ['rev-parse', 'HEAD'], { encoding: 'utf8' }).trim(),
      dirty: Boolean(execFileSync('git', ['status', '--porcelain'], { encoding: 'utf8' }).trim()),
    };
    const output = resolve(options.get('--output-dir'));
    // A new directory preserves previous evidence and prevents log replacement.
    mkdirSync(output, { mode: 0o700 });
    const receipt = {
      schemaVersion: 'agent-browser.local-validation.v1',
      source, cwd: process.cwd(), startedAt: new Date().toISOString(), state: 'running',
      checks: plan.checks.map((check, index) => ({ ...check, state: 'pending', log: `${index + 1}-${check.id}.log` })),
    };
    const save = () => {
      writeFileSync(join(output, 'receipt.tmp'), `${JSON.stringify(receipt, null, 2)}\n`, { mode: 0o600 });
      renameSync(join(output, 'receipt.tmp'), join(output, 'receipt.json'));
    };
    save();
    for (const check of receipt.checks) {
      const log = openSync(join(output, check.log), 'wx', 0o600);
      check.state = 'running';
      check.startedAt = new Date().toISOString();
      save();
      const started = performance.now();
      let result;
      try {
        result = spawnSync(check.command, check.args, {
          shell: false, stdio: ['ignore', log, log], timeout: check.timeoutMs,
        });
      } finally {
        closeSync(log);
      }
      check.durationMs = Math.round(performance.now() - started);
      check.finishedAt = new Date().toISOString();
      check.exitCode = result.status;
      check.signal = result.signal;
      check.errorCode = result.error?.code ?? null;
      check.state = result.status === 0 && !result.signal && !result.error ? 'success' : 'failure';
      save();
      console.log(`${check.id}: ${check.state} (exit ${check.exitCode ?? 'none'}, signal ${check.signal ?? 'none'}, error ${check.errorCode ?? 'none'})`);
    }
    receipt.state = receipt.checks.every((check) => check.state === 'success') ? 'success' : 'failure';
    receipt.finishedAt = new Date().toISOString();
    save();
    console.log(`Batch: ${receipt.state}; receipt: ${join(output, 'receipt.json')}`);
    process.exitCode = receipt.state === 'success' ? 0 : 1;
    }
  }
} catch (error) {
  console.error(boundPlan ? JSON.stringify({ code: error.code ?? 'validation_execution_unavailable' }) : error instanceof Error ? error.message : String(error));
  process.exitCode = 1;
}

function validatePlan(plan) {
  if (!plan || Object.keys(plan).some((key) => key !== 'checks') || !Array.isArray(plan.checks) || !plan.checks.length) {
    throw new Error('plan must contain a nonempty checks array');
  }
  const ids = new Set();
  for (const check of plan.checks) {
    if (!check || Object.keys(check).some((key) => !['id', 'command', 'args', 'timeoutMs'].includes(key))
      || typeof check.id !== 'string' || !/^[a-z0-9][a-z0-9-]{0,63}$/.test(check.id) || ids.has(check.id)
      || typeof check.command !== 'string' || !check.command || check.command.includes('\0')
      || !Array.isArray(check.args) || check.args.some((arg) => typeof arg !== 'string' || arg.includes('\0'))
      || (check.timeoutMs !== undefined && (!Number.isInteger(check.timeoutMs) || check.timeoutMs <= 0 || check.timeoutMs > 2_147_483_647))) {
      throw new Error('each check needs a unique kebab-case id, command, string args and optional positive timeoutMs');
    }
    ids.add(check.id);
  }
}
