import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { existsSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import test from 'node:test';

const runner = resolve('scripts/dev/run-validation.js');

function runPlan(t, checks) {
  const root = mkdtempSync(join(tmpdir(), 'validation-runner-'));
  t.after(() => rmSync(root, { recursive: true, force: true }));
  const plan = join(root, 'plan.json');
  const output = join(root, 'results');
  writeFileSync(plan, JSON.stringify({ checks: typeof checks === 'function' ? checks(root) : checks }));
  const result = spawnSync(process.execPath, [runner, '--plan', plan, '--output-dir', output], {
    encoding: 'utf8', timeout: 15_000,
  });
  return { root, output, result, receipt: () => JSON.parse(readFileSync(join(output, 'receipt.json'), 'utf8')) };
}
const nodeCheck = (id, source, extra = {}) => ({ id, command: process.execPath, args: ['-e', source], ...extra });

test('a passing final check cannot hide an earlier failure', (t) => {
  const run = runPlan(t, [
    nodeCheck('first', 'console.error("first failed"); process.exit(7)'),
    nodeCheck('second', 'console.log("second passed")'),
  ]);
  assert.equal(run.result.status, 1);
  const receipt = run.receipt();
  assert.equal(receipt.state, 'failure');
  assert.deepEqual(receipt.checks.map(({ id, state, exitCode }) => ({ id, state, exitCode })), [
    { id: 'first', state: 'failure', exitCode: 7 },
    { id: 'second', state: 'success', exitCode: 0 },
  ]);
  assert.match(readFileSync(join(run.output, receipt.checks[0].log), 'utf8'), /first failed/);
  assert.match(readFileSync(join(run.output, receipt.checks[1].log), 'utf8'), /second passed/);
  assert.match(receipt.source.head, /^[a-f0-9]{40}$/);
});

test('all passing checks keep literal arguments and file-backed large output', (t) => {
  const literal = '$(touch should-never-exist) ; `echo unsafe`';
  const run = runPlan(t, [
    { id: 'literal', command: process.execPath, args: ['-e', 'console.log(process.argv[1])', literal] },
    nodeCheck('large-output', 'process.stdout.write("x".repeat(2_000_000))'),
  ]);
  assert.equal(run.result.status, 0, run.result.stderr);
  const receipt = run.receipt();
  assert.equal(receipt.state, 'success');
  assert.equal(readFileSync(join(run.output, receipt.checks[0].log), 'utf8').trim(), literal);
  assert.equal(readFileSync(join(run.output, receipt.checks[1].log)).length, 2_000_000);
  assert.ok(run.result.stdout.length < 1500);
});

test('invalid later input starts no earlier child', (t) => {
  const run = runPlan(t, (root) => [
    { id: 'valid', command: process.execPath, args: ['-e', 'require("node:fs").writeFileSync(process.argv[1], "effect")', join(root, 'sentinel')] },
    nodeCheck('../invalid', 'process.exit(0)'),
  ]);
  assert.equal(existsSync(join(run.root, 'sentinel')), false);
  assert.equal(run.result.status, 1);
  assert.match(run.result.stderr, /unique kebab-case id/);
  assert.throws(() => run.receipt(), { code: 'ENOENT' });
});

test('launch failure cannot be hidden by a passing check', (t) => {
  const run = runPlan(t, [
    { id: 'missing', command: '/nonexistent/agent-browser-validation-test', args: [] },
    nodeCheck('passing', 'process.exit(0)'),
  ]);
  assert.equal(run.result.status, 1);
  const receipt = run.receipt();
  assert.equal(receipt.checks[0].errorCode, 'ENOENT');
  assert.equal(receipt.checks[0].state, 'failure');
  assert.equal(receipt.checks[1].state, 'success');
});

test('timeout fails the check and preserves the later result', (t) => {
  const run = runPlan(t, [
    nodeCheck('timeout', 'setInterval(() => {}, 1000)', { timeoutMs: 150 }),
    nodeCheck('passing', 'process.exit(0)'),
  ]);
  assert.equal(run.result.status, 1);
  const receipt = run.receipt();
  assert.equal(receipt.checks[0].errorCode, 'ETIMEDOUT');
  assert.equal(receipt.checks[0].state, 'failure');
  assert.equal(receipt.checks[1].state, 'success');
});

test('a signalled check never counts as success', (t) => {
  const run = runPlan(t, [nodeCheck('signal', 'process.kill(process.pid, "SIGTERM")')]);
  assert.equal(run.result.status, 1);
  assert.equal(run.receipt().state, 'failure');
  assert.notEqual(run.receipt().checks[0].signal, null);
});

test('an existing output directory is preserved rather than overwritten', (t) => {
  const run = runPlan(t, [nodeCheck('passing', 'process.exit(0)')]);
  assert.equal(run.result.status, 0);
  const before = readFileSync(join(run.output, 'receipt.json'), 'utf8');
  const again = spawnSync(process.execPath, [runner, '--plan', join(run.root, 'plan.json'), '--output-dir', run.output], { encoding: 'utf8' });
  assert.equal(again.status, 1);
  assert.match(again.stderr, /EEXIST/);
  assert.equal(readFileSync(join(run.output, 'receipt.json'), 'utf8'), before);
});
