#!/usr/bin/env node
/** Isolated browser continuity with legacy upgrade metadata. Never stops a workstation service. */
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { mkdirSync, readFileSync, readdirSync, readlinkSync, realpathSync, writeFileSync, existsSync, rmSync } from 'node:fs';
import { join } from 'node:path';
import { createMcpStdioClient, createSmokeContext } from './smoke-utils.js';

const binary = process.env.AGENT_BROWSER_SMOKE_AGENT_BROWSER_CMD;
const chrome = process.env.AGENT_BROWSER_SMOKE_BROWSER_EXECUTABLE;
assert(binary?.startsWith('/') && chrome?.startsWith('/'), 'Pin absolute candidate and browser paths');
const context = createSmokeContext({ prefix: 'ab-failed-upgrade-', sessionPrefix: 'failed-upgrade' });
context.env = Object.fromEntries(Object.entries(context.env).filter(([key]) => !key.startsWith('AGENT_BROWSER_') && !key.startsWith('XDG_')));
Object.assign(context.env, {
  AGENT_BROWSER_HOME: context.agentHome,
  AGENT_BROWSER_SOCKET_DIR: context.socketDir,
  AGENT_BROWSER_RUNTIME_HOST: '1',
  AGENT_BROWSER_EXECUTABLE_PATH: chrome,
  AGENT_BROWSER_SMOKE_AGENT_BROWSER_CMD: binary,
  XDG_RUNTIME_DIR: join(context.tempHome, 'runtime'),
  XDG_CONFIG_HOME: join(context.tempHome, 'config'),
  XDG_DATA_HOME: join(context.tempHome, 'data'),
});
mkdirSync(context.env.XDG_RUNTIME_DIR, { mode: 0o700 });
const client = createMcpStdioClient({ context, args: ['mcp', 'serve'], onFatal: () => {} });
const drain = join(context.tempHome, '.agent-browser', 'runtime-adoption', 'admission-drain.json');
let taskId;
const probeTaskIds = new Set();
let browser;
let owner;
const delay = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
function identity(pid) {
  try {
    const stat = readFileSync(`/proc/${pid}/stat`, 'utf8');
    const fields = stat.slice(stat.lastIndexOf(')') + 2).split(' ');
    return { pid, start: fields[19], argv: readFileSync(`/proc/${pid}/cmdline`, 'utf8'), executable: readlinkSync(`/proc/${pid}/exe`) };
  } catch { return null; }
}
function sameProcess(saved) {
  const current = saved && identity(saved.pid);
  return current?.start === saved?.start && current?.argv === saved?.argv && current?.executable === saved?.executable;
}
function ownedProcesses() {
  return readdirSync('/proc').filter((name) => /^\d+$/.test(name)).map((name) => identity(Number(name)))
    .filter((record) => {
      if (!record) return false;
      if (record.argv.includes(context.tempHome)) return true;
      try {
        const environment = readFileSync(`/proc/${record.pid}/environ`, 'utf8').split('\0');
        return environment.includes(`HOME=${context.tempHome}`) && environment.includes(`AGENT_BROWSER_HOME=${context.agentHome}`);
      } catch { return false; }
    });
}
async function waitFor(predicate, label, timeout = 5000) {
  const deadline = Date.now() + timeout;
  while (Date.now() < deadline) { if (await predicate()) return; await delay(25); }
  assert.fail(label);
}
async function call(name, arguments_) {
  const result = await client.send('tools/call', { name, arguments: arguments_ });
  const payload = JSON.parse(result.content.find((entry) => entry.type === 'text').text);
  assert.equal(payload.success, true, JSON.stringify(payload));
  return payload.data;
}
try {
  await client.send('initialize', { protocolVersion: '2025-06-18', capabilities: {}, clientInfo: { name: 'failed-upgrade-preservation', version: '1' } });
  client.notify('notifications/initialized');
  const opened = await call('browser_open', {
    serviceName: 'failed-upgrade-preservation-fixture', agentName: 'synthetic-preservation-worker', taskName: 'Preserve exact synthetic browser across repeated host exits',
    profile: 'failed-upgrade-preserved', profileMode: 'persistent', taskKey: 'preserved',
    url: 'data:text/html,<title>Preserved upgrade browser</title><h1>Preserved upgrade browser</h1>',
  });
  taskId = opened.taskId;
  const profile = join(context.agentHome, 'runtime-profiles', 'failed-upgrade-preserved', 'user-data');
  browser = ownedProcesses().find((record) => record.argv.includes(`--user-data-dir=${profile}`) && !record.argv.includes('--type='));
  assert(browser, 'Find exact owned browser process');
  owner = identity(Number(readFileSync(join(context.socketDir, 'runtime-host.pid'), 'utf8')));
  assert(owner && owner.executable === realpathSync(binary) && ownedProcesses().some((record) => record.pid === owner.pid), 'Pin exact owned runtime host');
  const endpoint = readFileSync(join(profile, 'DevToolsActivePort'), 'utf8').split('\n');
  const versionUrl = `http://127.0.0.1:${Number(endpoint[0])}/json/version`;
  const before = await (await fetch(versionUrl)).json();
  const profileLock = readlinkSync(join(profile, 'SingletonLock'));
  assert(profileLock.endsWith(`-${browser.pid}`), 'Profile control belongs to exact owned browser');
  const completedCycles = [];
  for (let cycle = 1; cycle <= 2; cycle += 1) {
    mkdirSync(join(drain, '..'), { recursive: true });
    const drainBytes = cycle === 1
      ? JSON.stringify({ schemaVersion: 'agent-browser.runtime-adoption.v1', transactionId: 'failed-upgrade-fixture', candidateGenerationId: 'failed-candidate', transactionRevision: 7, recordedAt: new Date().toISOString() })
      : 'invalid-json';
    writeFileSync(drain, drainBytes);
    const probe = await call('browser_open', { serviceName: 'failed-upgrade-preservation-fixture', agentName: 'synthetic-admission-probe', taskName: 'Verify legacy upgrade metadata allows a new disposable task', profileMode: 'disposable', taskKey: `admitted-${cycle}`, url: 'about:blank' });
    probeTaskIds.add(probe.taskId);
    await call('browser_close', { taskId: probe.taskId });
    probeTaskIds.delete(probe.taskId);
    owner = identity(Number(readFileSync(join(context.socketDir, 'runtime-host.pid'), 'utf8')));
    assert(owner && owner.executable === realpathSync(binary) && ownedProcesses().some(record => record.pid === owner.pid), 'Pin the exact current owned host for each cycle');
    assert(!completedCycles.some(previous => previous.hostPid === owner.pid && previous.hostStart === owner.start), 'Continuation must create a new host process instance');
    const started = performance.now();
    assert(sameProcess(owner), 'Recheck runtime identity before its graceful signal');
    process.kill(owner.pid, 'SIGTERM');
    await waitFor(() => !sameProcess(owner), 'Owned runtime host must exit within five seconds');
    // Negative control breaks only this fixture's browser to prove the PID oracle.
    if (process.argv.includes('--break-preservation')) {
      assert(sameProcess(browser), 'Pin fixture browser before the negative control');
      process.kill(browser.pid, 'SIGTERM');
      await waitFor(() => !sameProcess(browser), 'Negative-control browser must exit');
    }
    assert(sameProcess(browser), 'Preserve exact browser PID and process instance through host exit');
    const after = await (await fetch(versionUrl)).json();
    assert.equal(after.webSocketDebuggerUrl, before.webSocketDebuggerUrl, 'Keep exact browser CDP endpoint');
    assert.equal(readlinkSync(join(profile, 'SingletonLock')), profileLock, 'Preserved browser keeps occupied profile control');
    assert.equal(readFileSync(drain, 'utf8'), drainBytes, 'Host shutdown must not silently clear failed-upgrade drain');
    for (const suffix of ['pid', 'identity', 'version', 'exe', 'token', 'sock']) {
      assert(!existsSync(join(context.socketDir, `runtime-host.${suffix}`)), `Retire owned endpoint ${suffix}`);
    }
    const elapsedMs = Math.round(performance.now() - started);
    completedCycles.push({ cycle, hostPid: owner.pid, hostStart: owner.start, elapsedMs });
    // Continue while legacy metadata remains, including across host restart.
    const continued = await call('browser_continue', { taskId });
    assert.equal(continued.taskId, taskId, 'Keep logical task identity after owner restart');
    assert(sameProcess(browser), 'Continuation must reuse preserved browser');
  }
  await call('browser_close', { taskId });
  taskId = null;
  await waitFor(() => !sameProcess(browser), 'Explicit task close retires exact owned browser');
  console.log(JSON.stringify({ success: true, binarySha256: createHash('sha256').update(readFileSync(binary)).digest('hex'), completedHostExitCycles: completedCycles.length, cycles: completedCycles, browserPidPreserved: true, cdpEndpointPreserved: true, occupiedProfilePreserved: true, continuationPassed: true, scope: 'two sequential graceful owner exits with nonblocking legacy upgrade metadata; no workstation-wide or multi-binary-generation shutdown claim' }));
} finally {
  if (existsSync(drain)) rmSync(drain);
  for (const probeTaskId of probeTaskIds) { try { await call('browser_close', { taskId: probeTaskId }); } catch {} }
  if (taskId) { try { await call('browser_close', { taskId }); } catch {} }
  client.close();
  for (const process_ of ownedProcesses().filter((record) => record.executable === realpathSync(binary))) {
    if (sameProcess(process_)) process.kill(process_.pid, 'SIGTERM');
  }
  await waitFor(() => ownedProcesses().length === 0, 'Zero exact fixture-owned process residue');
  console.log('Fixture census: zero owned process residue');
  context.cleanupTempHome();
}
