#!/usr/bin/env node

import { verifyPresubmit } from '../lib/presubmit-verifier.js';

try {
  const selection = parseEnvironmentJson('VALIDATION_SELECTION_JSON');
  const results = parseEnvironmentJson('VALIDATION_RESULTS_JSON');
  const receipt = verifyPresubmit({ selection, results });
  process.stdout.write(`${JSON.stringify(receipt, null, 2)}\n`);
} catch (error) {
  process.stderr.write(`${error instanceof Error ? error.message : String(error)}\n`);
  process.exitCode = 1;
}

function parseEnvironmentJson(name) {
  const value = process.env[name];
  if (!value) throw new Error(`${name} is required`);
  try {
    return JSON.parse(value);
  } catch (error) {
    throw new Error(`${name} is not valid JSON: ${error instanceof Error ? error.message : String(error)}`);
  }
}
