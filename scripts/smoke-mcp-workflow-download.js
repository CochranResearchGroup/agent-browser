#!/usr/bin/env node
/** Existing-profile file retrieval uses current process evidence, not owner records. */
import assert from 'node:assert/strict';
import { mkdirSync, readFileSync, readdirSync } from 'node:fs';
import { join } from 'node:path';
import { execFileSync } from 'node:child_process';
import { createMcpStdioClient, createSmokeContext, runCli } from './smoke-utils.js';

assert(process.env.AGENT_BROWSER_SMOKE_AGENT_BROWSER_CMD, 'Pin the candidate executable');
const context = createSmokeContext({ prefix: 'ab-download-', sessionPrefix: 'download' });
const executable = process.env.AGENT_BROWSER_SMOKE_BROWSER_EXECUTABLE;
assert(executable, 'Pin a reviewed Linux browser executable');
const env = Object.fromEntries(
  Object.entries(context.env).filter(
    ([key]) => !key.startsWith('AGENT_BROWSER_') && !key.startsWith('XDG_')
  )
);
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
  const value = JSON.parse(result.content.find((block) => block.type === 'text').text);
  assert.equal(value.success, true, JSON.stringify(value));
  return value.data;
}
const tasks = [];
async function call(name, arguments_) {
  return payload(await client.send('tools/call', { name, arguments: arguments_ }));
}
function ownedProcesses() {
  return readdirSync('/proc')
    .filter((name) => /^\d+$/.test(name))
    .filter((pid) => {
      if (Number(pid) === process.pid) return false;
      try {
        return readFileSync(`/proc/${pid}/environ`)
          .toString()
          .split('\0')
          .includes(`AGENT_BROWSER_HOME=${context.agentHome}`);
      } catch {
        return false;
      }
    })
    .map(Number);
}
try {
  await client.send('initialize', {
    protocolVersion: '2025-06-18',
    capabilities: {},
    clientInfo: { name: 'workflow-fixture', version: '0' },
  });
  client.notify('notifications/initialized');
  const created = await runCli(context, [
    '--json', 'runtime', 'create', 'issue284-fixture', '--browser-family', 'chromium',
  ]);
  assert.equal(JSON.parse(created.stdout).created, true);
  const task = await call('browser_open', {
    profile: 'issue284-fixture',
    profileMode: 'existing',
    taskKey: 'download',
    url: 'data:text/html,<title>Download fixture</title><a id="download" download="fixture.txt" href="data:text/plain,issue284-synthetic">Download</a>',
  });
  tasks.push(task);
  const identity = [task.taskId, task.browserId, task.tabId];
  const snapshot = await call('browser_snapshot', { taskId: task.taskId });
  assert(JSON.stringify(snapshot).includes('Download'));
  if (process.env.AGENT_BROWSER_SMOKE_STALE_CUSTODY === '1' ||
      process.env.AGENT_BROWSER_SMOKE_MALFORMED_CUSTODY === '1') {
    // Fault injection touches only this disposable fixture's SQLite document.
    // The real task/browser remain live; historical rows must not veto reads.
    const observedPid = Number(execFileSync('python3', ['-c', `
import json, sqlite3, sys, uuid
connection = sqlite3.connect(sys.argv[1])
state = json.loads(connection.execute("SELECT json FROM state_documents WHERE kind='browser_session_state'").fetchone()[0])
browser = state['browsers'][sys.argv[2]]
assignment = dict(assignmentId='synthetic-assignment', registrationId='synthetic-registration',
    poolId='synthetic-pool', desktopId=str(uuid.uuid4()), generation=1, state='active')
records = {}
for pid, published in [(4000000000, True), (browser['pid'], False)]:
    intent = dict(intentId=str(uuid.uuid4()), profileId=browser['profileId'], assignment=assignment)
    records[intent['intentId']] = dict(schemaVersion=1, intent=intent,
        observedBrowserId=browser['id'], observedPid=pid, published=published)
if sys.argv[3] == "malformed":
    records = {"broken": {"old": "history"}}
connection.execute("INSERT INTO state_documents(kind,schema_version,json) VALUES (?,?,?)",
    ('browser_launch_custody','agent-browser.browser-launch-custody.v1',json.dumps(dict(records=records))))
connection.commit()
print(browser['pid'])
`, join(context.agentHome, 'service', 'runtime.sqlite3'), task.browserId, process.env.AGENT_BROWSER_SMOKE_MALFORMED_CUSTODY === '1' ? 'malformed' : 'stale']).toString().trim());
    const startToken = () => {
      const stat = readFileSync(`/proc/${observedPid}/stat`, 'utf8');
      return stat.slice(stat.lastIndexOf(')') + 2).split(' ')[19];
    };
    const before = startToken();
    const results = await Promise.all([
      call('browser_continue', { taskId: task.taskId }),
      call('browser_continue', { taskId: task.taskId }),
    ]);
    for (const result of results) {
      assert.deepEqual([result.taskId, result.browserId, result.tabId], identity);
    }
    assert.equal(startToken(), before, 'continuation retains the exact live process');
    console.log('Concurrent continuation ignores historical custody and preserves the live task/process');
  }
  const destination = join(context.tempHome, 'fixture.txt');
  await call('browser_command', {
    taskId: task.taskId,
    action: 'download',
    params: { selector: '#download', path: destination },
  });
  assert.equal(readFileSync(destination, 'utf8'), 'issue284-synthetic');
  const continued = await call('browser_continue', { taskId: task.taskId });
  assert.deepEqual([continued.taskId, continued.browserId, continued.tabId], identity);
  console.log('Existing-profile download bytes and task continuity passed');
} finally {
  for (const task of tasks.reverse()) {
    try {
      await call('browser_close', { taskId: task.taskId });
    } catch (error) {
      console.error(`Fixture task cleanup failed: ${error.message}`);
    }
  }
  clearTimeout(timeout);
  client.close();
  // Stop only processes positively bound to this fixture's exact private home.
  for (const pid of ownedProcesses()) {
    try {
      if (ownedProcesses().includes(pid)) process.kill(pid, 'SIGTERM');
    } catch {
      /* Process exit during the ownership census is harmless. */
    }
  }
  const cleanupDeadline = Date.now() + 5000;
  while (ownedProcesses().length > 0 && Date.now() < cleanupDeadline) {
    await new Promise((resolve) => setTimeout(resolve, 50));
  }
  assert.equal(
    ownedProcesses().length,
    0,
    `Fixture processes remain; retained cleanup obligation at ${context.tempHome}`
  );
  context.cleanupTempHome();
  console.log('Fixture process census: zero owned residue');
}
