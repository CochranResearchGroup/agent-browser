#!/usr/bin/env node
import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { createHash } from 'node:crypto';
import { existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { createServer } from 'node:net';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import { createInterface } from 'node:readline';

const binary = resolve(process.env.AGENT_BROWSER_SMOKE_AGENT_BROWSER_CMD || 'cli/target/ci/agent-browser');
const root = mkdtempSync(join(tmpdir(), 'ab-request-rejection-'));
const home = join(root, 'home');
const agentHome = join(home, '.agent-browser');
const socketDir = join(root, 's');
const supervisor = join(root, 'supervisor');
for (const path of [home, socketDir, join(supervisor, 'manifests')]) {
  mkdirSync(path, { recursive: true, mode: 0o700 });
}
const portProbe = createServer();
await new Promise(resolve => portProbe.listen(0, '127.0.0.1', resolve));
const port = portProbe.address().port;
await new Promise(resolve => portProbe.close(resolve));
writeFileSync(join(supervisor, 'manifests', 'rejection.json'), JSON.stringify({
  schemaVersion: 'agent-browser.session-supervisor.v1', session: 'rejection',
  executablePath: binary, executableSha256: createHash('sha256').update(readFileSync(binary)).digest('hex'),
  streamPort: port, runtimeProfile: 'rejection', provenance: { packageVersion: '0.28.0',
    installedAt: '2026-09-05T00:00:00Z', installedBy: 'no-launch rejection smoke' },
}), { mode: 0o600 });
const env = { ...process.env, HOME: home, AGENT_BROWSER_HOME: agentHome,
    AGENT_BROWSER_SOCKET_DIR: socketDir, AGENT_BROWSER_SESSION_SUPERVISOR_ROOT: supervisor,
    AGENT_BROWSER_RUNTIME_HOST: '1', AGENT_BROWSER_RUNTIME_ENVIRONMENT: 'development',
    AGENT_BROWSER_EXTERNAL_BROWSER_DISCOVERY: 'disabled', AGENT_BROWSER_SERVICE_RECONCILE_INTERVAL_MS: '0' };
const child = spawn(binary, ['session', 'supervisor', 'run-host'], {
  env,
  stdio: ['ignore', 'pipe', 'pipe'],
});
let output = '';
child.stdout.on('data', bytes => { output += bytes; });
child.stderr.on('data', bytes => { output += bytes; });
const exited = new Promise(resolve => child.once('exit', resolve));
async function assertRejection(payload, code) {
  assert.equal(payload.failure?.code, code);
  assert.equal(payload.failure?.effectState, 'no_effect');
  assert.equal(payload.failure?.phase, 'ingress_validation');
  assert.equal(payload.failure?.retryDisposition, 'do_not_retry');
  assert.equal(typeof payload.id, 'string');
  const deadline = Date.now() + 5000;
  let matches = [];
  // Delivery is asynchronous. Observe its bounded completion before checking
  // correlation; interruption durability is a separate acceptance requirement.
  do {
    const journalPath = join(agentHome, 'service', 'failure-journal.jsonl');
    const text = existsSync(journalPath) ? readFileSync(journalPath, 'utf8') : '';
    const completeLines = text.slice(0, text.lastIndexOf('\n') + 1);
    matches = completeLines.split('\n').filter(Boolean).map(line => JSON.parse(line))
      .filter(row => row.references?.requestId === payload.id);
    if (matches.length > 0) break;
    await new Promise(resolve => setTimeout(resolve, 20));
  } while (Date.now() < deadline);
  assert.equal(matches.length, 1, 'response must join exactly one durable failure within 5 seconds');
  assert.equal(matches[0].code, payload.failure.code);
  assert.equal(matches[0].stage, payload.failure.phase);
}
try {
  const deadline = Date.now() + 15_000;
  while (!existsSync(join(socketDir, 'rejection.stream'))) {
    assert(child.exitCode === null, output);
    assert(Date.now() < deadline, `host readiness timeout: ${output}`);
    await new Promise(resolve => setTimeout(resolve, 25));
  }
  for (const [body, code] of [
    ['{}', 'missing_action'],
    [JSON.stringify({ action: 'tab_new', runtimeEnvironmentId: 'forged', serviceName: 'RejectionSmoke',
      agentName: 'test', taskName: 'validate' }), 'unknown_field'],
    ['{', 'invalid_request'],
  ]) {
    const response = await fetch(`http://127.0.0.1:${port}/api/service/request`, {
      method: 'POST', headers: { 'content-type': 'application/json' }, body,
      signal: AbortSignal.timeout(5000),
    });
    assert.equal(response.status, 400);
    const payload = await response.json();
    assert.equal(payload.success, false);
    await assertRejection(payload, code);
  }
  const mcp = spawn(binary, ['--session', 'rejection', 'mcp', 'serve'], {
    env, stdio: ['pipe', 'pipe', 'pipe'],
  });
  const mcpExited = new Promise(resolve => mcp.once('exit', resolve));
  const lines = createInterface({ input: mcp.stdout });
  mcp.stderr.on('data', bytes => { output += bytes; });
  function send(id, method, params) {
    return new Promise((resolve, reject) => {
      const timer = setTimeout(() => {
        lines.removeListener('line', receive);
        reject(new Error(`MCP request ${id} timed out`));
      }, 5000);
      function receive(line) {
        const reply = JSON.parse(line);
        if (reply.id !== id) return;
        clearTimeout(timer);
        lines.removeListener('line', receive);
        resolve(reply);
      }
      lines.on('line', receive);
      mcp.stdin.write(`${JSON.stringify({ jsonrpc: '2.0', id, method, params })}\n`);
    });
  }
  try {
    const initialized = await send(1, 'initialize', { protocolVersion: '2025-06-18',
      capabilities: {}, clientInfo: { name: 'rejection-smoke', version: '1' } });
    assert(initialized.result?.capabilities?.tools);
    mcp.stdin.write(`${JSON.stringify({ jsonrpc: '2.0', method: 'notifications/initialized' })}\n`);
    for (const [id, args, code] of [[2, {}, 'missing_action'],
      [3, { action: 'tab_new', runtimeEnvironmentId: 'forged' }, 'unknown_field']]) {
      const reply = await send(id, 'tools/call', { name: 'service_request', arguments: args });
      assert.equal(reply.error?.code, -32602);
      await assertRejection({ id: reply.error.data.requestId, failure: reply.error.data.failure }, code);
    }
  } finally {
    lines.close();
    if (mcp.exitCode === null) mcp.kill('SIGTERM');
    await mcpExited;
  }
  const state = JSON.parse(readFileSync(join(agentHome, 'service', 'state.json'), 'utf8'));
  assert.equal(Object.keys(state.jobs ?? {}).length, 0);
  assert.equal(Object.keys(state.browsers ?? {}).length, 0);
  console.log('Service request rejection no-launch smoke passed: five HTTP/MCP responses joined to durable failures');
} finally {
  if (child.exitCode === null) child.kill('SIGTERM');
  await exited;
  rmSync(root, { recursive: true, force: true });
}
