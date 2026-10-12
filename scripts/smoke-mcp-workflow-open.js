#!/usr/bin/env node
/** Public MCP workflow regression; every process and profile uses a disposable namespace. */
import assert from 'node:assert/strict';
import { mkdirSync, readFileSync, writeFileSync, existsSync, readdirSync } from 'node:fs';
import { join } from 'node:path';
import { spawn } from 'node:child_process';
import { createMcpStdioClient, createSmokeContext, runCli } from './smoke-utils.js';

assert(process.env.AGENT_BROWSER_SMOKE_AGENT_BROWSER_CMD, 'Pin the candidate executable');
const context = createSmokeContext({ prefix: 'ab-workflow-', sessionPrefix: 'workflow' });
const executable = process.env.AGENT_BROWSER_SMOKE_BROWSER_EXECUTABLE;
assert(executable, 'Pin a reviewed Linux browser executable');
const env = Object.fromEntries(Object.entries(context.env).filter(([key]) =>
  !key.startsWith('AGENT_BROWSER_') && !key.startsWith('XDG_')));
Object.assign(env, {
  AGENT_BROWSER_HOME: context.agentHome,
  AGENT_BROWSER_SOCKET_DIR: context.socketDir,
  AGENT_BROWSER_RUNTIME_HOST: '1',
  AGENT_BROWSER_EXECUTABLE_PATH: executable,
  AGENT_BROWSER_SMOKE_AGENT_BROWSER_CMD: process.env.AGENT_BROWSER_SMOKE_AGENT_BROWSER_CMD,
  XDG_RUNTIME_DIR: join(context.tempHome, 'runtime'),
  XDG_CONFIG_HOME: join(context.tempHome, 'config'),
  XDG_DATA_HOME: join(context.tempHome, 'data'),
});
mkdirSync(env.XDG_RUNTIME_DIR, { mode: 0o700 });
context.env = env;
console.log(`Fixture namespace: ${context.agentHome}`);
const client = createMcpStdioClient({ context, args: ['mcp', 'serve'], onFatal: () => {} });
const timeout = setTimeout(() => client.rejectPending('Workflow fixture timed out'), 45000);
function payload(result) {
  assert.equal(result.isError, false, JSON.stringify(result));
  const value = JSON.parse(result.content.find(block => block.type === 'text').text);
  assert.equal(value.success, true, JSON.stringify(value));
  return value.data;
}
const tasks = [];
let borrowedRoot;
async function call(name, arguments_) {
  return payload(await client.send('tools/call', { name, arguments: arguments_ }));
}
function ownedProcesses() {
  return readdirSync('/proc').filter(name => /^\d+$/.test(name)).filter(pid => {
    if (Number(pid) === process.pid) return false;
    try {
      return readFileSync(`/proc/${pid}/environ`).toString().split('\0')
        .includes(`AGENT_BROWSER_HOME=${context.agentHome}`);
    } catch { return false; }
  }).map(Number);
}
try {
  await client.send('initialize', {
    protocolVersion: '2025-06-18', capabilities: {},
    clientInfo: { name: 'workflow-fixture', version: '0' },
  });
  client.notify('notifications/initialized');
  const request = { profile: 'workflow-fixture', profileMode: 'persistent',
    taskKey: 'alice', url: 'data:text/html,<title>Alice fixture</title><input id="draft" aria-label="Draft">' };
  const task = await call('browser_open', request);
  tasks.push(task);
  assert.equal(task.profileId, request.profile);
  assert.equal(typeof task.taskId, 'string');
  assert(task.taskId.length > 0);
  assert.equal(typeof task.browserId, 'string');
  assert.equal(typeof task.tabId, 'string');
  assert.equal(task.url, request.url);
  await call('browser_fill', { taskId: task.taskId, selector: '#draft', value: 'Alice draft' });
  const bobRequest = { ...request, taskKey: 'bob',
    url: 'data:text/html,<title>Bob fixture</title><input id="draft" aria-label="Draft">' };
  const bob = await call('browser_open', bobRequest);
  tasks.push(bob);
  assert.equal(bob.browserId, task.browserId, 'same-profile tasks share the healthy browser');
  assert.notEqual(bob.taskId, task.taskId);
  assert.notEqual(bob.tabId, task.tabId);
  await call('browser_fill', { taskId: bob.taskId, selector: '#draft', value: 'Bob draft' });
  await assert.rejects(call('browser_handoff', { taskId: task.taskId }),
    /remote_view_public_origin_missing/);
  const alicePage = await call('browser_snapshot', { taskId: task.taskId });
  assert(JSON.stringify(alicePage).includes('Alice draft'), 'Bob must preserve Alice page edits');
  assert(!JSON.stringify(alicePage).includes('Bob draft'), 'Alice actions must not target Bob');
  const replay = await call('browser_open', request);
  assert.equal(replay.taskId, task.taskId, 'replay retains the task');
  assert.equal(replay.browserId, task.browserId, 'replay retains the browser');
  assert.equal(replay.tabId, task.tabId, 'replay must not allocate a duplicate tab');
  const replayPage = await call('browser_snapshot', { taskId: task.taskId });
  assert(JSON.stringify(replayPage).includes('Alice draft'), 'replay must preserve existing page edits');
  const continued = await call('browser_continue', { taskId: task.taskId });
  assert.equal(continued.taskId, task.taskId);
  assert.equal(continued.browserId, task.browserId);
  assert.equal(continued.tabId, task.tabId);
  assert.equal(continued.url, request.url);
  const continuedPage = await call('browser_snapshot', { taskId: task.taskId });
  assert(JSON.stringify(continuedPage).includes('Alice draft'), 'continuation preserves page edits');
  const initialTabs = await call('browser_tab', {taskId: task.taskId, operation: 'list'});
  assert.equal(initialTabs.tabs.length, 1);
  assert.equal(initialTabs.tabs[0].tabId, task.tabId, 'task list excludes Bob');
  await assert.rejects(call('browser_tab', {taskId: task.taskId, operation: 'select', tabId: bob.tabId}),
    /browser_tab_not_owned/, 'Alice cannot select Bob tab');
  const extra = await call('browser_tab', {taskId: task.taskId, operation: 'new', tabKey: 'notes', url: request.url});
  const extraReplay = await call('browser_tab', {taskId: task.taskId, operation: 'new', tabKey: 'notes', url: request.url});
  assert.equal(extraReplay.tabId, extra.tabId, 'replayed tab key does not create another tab');
  const aliceTabs = await call('browser_tab', {taskId: task.taskId, operation: 'list'});
  assert.equal(aliceTabs.tabs.length, 2);
  await call('browser_tab', {taskId: task.taskId, operation: 'select', tabId: task.tabId});
  assert(JSON.stringify(await call('browser_snapshot', {taskId: task.taskId})).includes('Alice draft'));
  await call('browser_tab', {taskId: task.taskId, operation: 'close', tabId: extra.tabId});
  assert.equal((await call('browser_tab', {taskId: task.taskId, operation: 'list'})).tabs.length, 1);
  await call('browser_close', { taskId: task.taskId });
  tasks.splice(tasks.indexOf(task), 1);
  const survivingPage = await call('browser_snapshot', { taskId: bob.taskId });
  assert(JSON.stringify(survivingPage).includes('Bob draft'), 'closing Alice preserves usable Bob work');
  const closed = await call('browser_close', { taskId: bob.taskId });
  tasks.splice(tasks.indexOf(bob), 1);
  assert.equal(closed.disposition, 'browser_closed', 'the final owned task releases its browser');
  console.log('Persistent MCP workflow sharing, page-preserving replay and scoped close passed');

  const created = JSON.parse((await runCli(context,
    ['--json', 'runtime', 'create', 'borrowed-fixture', '--browser-family', 'chromium'])).stdout);
  // Legacy supported runtime creation left its profile only in user configuration.
  // Reproduce that public configuration without altering any operator profile.
  const statePath = join(context.agentHome, 'service', 'state.json');
  const legacyState = JSON.parse(readFileSync(statePath, 'utf8'));
  delete legacyState.profiles['borrowed-fixture'];
  writeFileSync(statePath, `${JSON.stringify(legacyState)}\n`);
  borrowedRoot = spawn(executable, [
    '--headless', '--no-sandbox', '--disable-dev-shm-usage', '--disable-background-networking',
    '--password-store=basic', '--remote-debugging-port=0',
    `--user-data-dir=${created.userDataDir}`, 'about:blank',
  ], { env, stdio: 'ignore' });
  const portFile = join(created.userDataDir, 'DevToolsActivePort');
  const deadline = Date.now() + 10000;
  while (!existsSync(portFile) && Date.now() < deadline && borrowedRoot.exitCode === null) {
    await new Promise(resolve => setTimeout(resolve, 50));
  }
  assert(existsSync(portFile), 'the fixture borrowed browser must launch before testing reuse');
  const borrowed = await call('browser_open', { ...request, profile: 'borrowed-fixture',
    profileMode: 'existing', taskKey: 'borrowed-task' });
  tasks.push(borrowed);
  const released = await call('browser_close', { taskId: borrowed.taskId });
  tasks.splice(tasks.indexOf(borrowed), 1);
  assert.equal(released.disposition, 'browser_preserved', 'closing a borrowed task preserves its browser');
  process.kill(borrowedRoot.pid, 0);
  assert.equal(borrowedRoot.exitCode, null, 'the original borrowed browser process remains live');
  console.log('Existing-profile workflow preserves the borrowed browser');
  await assert.rejects(call('browser_open', { ...request, profile: 'missing-fixture',
    profileMode: 'existing', taskKey: 'missing' }), /browser_profile_not_found:missing-fixture/);
  const temporary = await call('browser_open', { profileMode: 'disposable',
    taskKey: 'temporary', url: request.url });
  tasks.push(temporary);
  assert.notEqual(temporary.profileId, request.profile);
  assert.notEqual(temporary.browserId, borrowed.browserId);
  const temporaryReplay = await call('browser_open', { profileMode: 'disposable',
    taskKey: 'temporary', url: request.url });
  assert.equal(temporaryReplay.taskId, temporary.taskId);
  assert.equal(temporaryReplay.tabId, temporary.tabId);
  const ended = await call('browser_close', { taskId: temporary.taskId });
  tasks.splice(tasks.indexOf(temporary), 1);
  assert.equal(ended.disposition, 'browser_closed');
  process.kill(borrowedRoot.pid, 0);
  console.log('Missing-profile and disposable-task workflows passed');
} finally {
  for (const task of tasks.reverse()) {
    try { await call('browser_close', { taskId: task.taskId }); }
    catch (error) { console.error(`Fixture task cleanup failed: ${error.message}`); }
  }
  clearTimeout(timeout);
  client.close();
  if (borrowedRoot?.exitCode === null) borrowedRoot.kill('SIGTERM');
  // Stop only processes positively bound to this fixture's exact private home.
  for (const pid of ownedProcesses()) {
    try {
      if (ownedProcesses().includes(pid)) process.kill(pid, 'SIGTERM');
    } catch { /* Process exit during the ownership census is harmless. */ }
  }
  const cleanupDeadline = Date.now() + 5000;
  while (ownedProcesses().length > 0 && Date.now() < cleanupDeadline) {
    await new Promise(resolve => setTimeout(resolve, 50));
  }
  assert.equal(ownedProcesses().length, 0,
    `Fixture processes remain; retained cleanup obligation at ${context.tempHome}`);
  context.cleanupTempHome();
  console.log('Fixture process census: zero owned residue');
}
