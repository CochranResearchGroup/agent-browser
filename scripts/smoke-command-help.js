#!/usr/bin/env node
/** Unknown help requests must fail cheaply before daemon startup. */
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { createSmokeContext } from './smoke-utils.js';
const binary = process.env.AGENT_BROWSER_SMOKE_AGENT_BROWSER_CMD;
assert(binary, 'Pin the candidate executable');
const context = createSmokeContext({ prefix: 'ab-help-', sessionPrefix: 'help' });
try {
  for (const args of [
    ['runtime-profile', '--help'],
    ['runtime', 'imaginary', '--help'],
    ['get', 'imaginary', '--help'],
    ['service', 'imaginary', '--help'],
  ]) {
    const result = spawnSync(binary, args, { env: context.env, encoding: 'utf8', timeout: 10000 });
    assert.equal(result.status, 1, `${args.join(' ')} fails with the correct status`);
    const output = result.stdout + result.stderr;
    assert(Buffer.byteLength(output) <= 2000, 'Unknown help remains bounded');
    const recovery = args[0].startsWith('runtime') ? 'runtime create' : `${args[0]} --help`;
    assert(output.includes(recovery), 'Recovery names the supported command');
    const json = spawnSync(binary, ['--json', ...args], {
      env: context.env,
      encoding: 'utf8',
      timeout: 10000,
    });
    assert.equal(json.status, 1);
    const error = JSON.parse(json.stdout);
    assert.equal(error.success, false);
    assert.equal(
      error.type,
      args[0] === 'runtime-profile' ? 'unknown_command' : 'unknown_subcommand'
    );
    assert(error.error.includes(recovery));
  }
  const focused = spawnSync(binary, ['runtime', 'create', '--help'], {
    env: context.env,
    encoding: 'utf8',
    timeout: 10000,
  });
  assert.equal(focused.status, 0);
  assert(
    Buffer.byteLength(focused.stdout + focused.stderr) <= 4000,
    'Runtime create help remains focused'
  );
  assert(focused.stdout.includes('runtime create'));
  console.log('Unknown-command and focused runtime-create help passed');
} finally {
  context.cleanupTempHome();
}
